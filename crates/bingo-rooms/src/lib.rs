//! Rooms: a room *is* a session nobody answers (ADR-0011 §1), and a post into
//! it reaches every member. This plugin owns the noun the kernel refuses to
//! and adds no machinery of its own: a room is a `Log` session under the
//! person's, its membership is an `Event::Extension` in its own journal, a
//! post is a `User` item in it, and the fan-out is `deliver`.
//!
//! One command, four verbs and one hook:
//!
//! - `/room` lists the rooms under this session; `/room design reviewer scout`
//!   opens `#design` under it, or resets who is in the one that stands, and
//!   `/room close design` ends one. A person's door keeps the reset lever: a
//!   person outranks the protocol (ADR-0025 §4).
//! - `OpenRoom` is that same door with an agent on the other side of it
//!   (ADR-0021): the room hangs under the caller, or — with `shared` — under
//!   the caller's parent, which is the whole of who will hear it. It is opened
//!   for one purpose and a name that stands is refused (ADR-0053 §1–2).
//! - `Seat`, `Unseat` and `CloseRoom` move the roster of a room that stands and
//!   end it. Each is a post the room reads, and only the session a room hangs
//!   under and whoever opened it may call one (ADR-0053 §3–5).
//! - The hook seats the rooms `.bingo/team.json` declares when a person's own
//!   session opens, and watches every journal: a room announces itself, an
//!   extension says who is in it, and a user item in one is a post to fan out.
//!   Nothing here opens that file: the plugin that owns it is its one parser,
//!   and its `rooms` key is asked for by service (ADR-0031).
//!
//! A post also owes (ADR-0022): `@name` opens a debt the member's next post
//! closes, one bounded chaser nudges whoever stays silent, and `/room` and a
//! card on the room's parent show what stands.
//!
//! Every seat has an ear (ADR-0029): patience 0 is a live one — every post
//! wakes it, the default — and thirty seconds or more a patient one, whose
//! posts wait for the seat's next turn and wake it once when they have waited
//! long enough. The three doors declare it, `Listen` retunes your own, and
//! `@name` pierces either.
//!
//! Nothing here keeps a roster or a debt beside the journal: what the hook
//! holds in memory is a fold of the frames it saw, `/room` reads a membership
//! back out of the room it belongs to, and the mentions are derived from the
//! room's own posts every time they are asked for.

mod chase;
mod command;
mod cursor;
mod deadline;
mod door;
mod ear;
pub mod guide;
mod hook;
mod listen;
mod mentions;
use bingo_collab::name;

mod control;
mod owed;
mod placement;
mod post;
mod reader;
mod recovery;
mod room;
mod roster;
mod seat;
mod serial;
mod service;
pub use service::RoomWriter;
mod team;
mod tool;

use std::sync::Arc;

use async_trait::async_trait;
use bingo_sdk::{
    ClientIdentity, Command, ContextContributor, Contribution, Hook, Plugin, PluginError,
    PluginManifest, Registrar, Tool,
};

pub use command::RoomCommand;
pub use ear::{Ear, Listener, Seat};
pub use hook::RoomsHook;
pub use listen::ListenTool;
pub use reader::Reader;
pub use room::Room;
pub use seat::{Opening, seat};
pub use team::{Entry, TeamError};
pub use tool::{CloseRoomTool, OpenRoomTool, SeatTool, UnseatTool};

/// This plugin's id: the owner of a room's key, and the plugin a room's
/// membership is published under.
pub use bingo_collab::room::PLUGIN;

/// The surface a room's messages come from; a person's say `tui` or `print`.
pub const SURFACE: &str = "room";

static MANIFEST: PluginManifest = PluginManifest {
    id: PLUGIN,
    version: env!("CARGO_PKG_VERSION"),
    uses: &[
        bingo_services::team::KEY.name(),
        bingo_sdk::store::reader::KEY.name(),
    ],
    provides: &[
        "command:room",
        "hook:rooms",
        "tool:OpenRoom",
        "tool:Seat",
        "tool:Unseat",
        "tool:CloseRoom",
        "tool:Listen",
        "context:rooms",
        bingo_services::room_post::CAPABILITY,
        bingo_services::rooms_control::CAPABILITY,
        "service:bingo.rooms.pages",
    ],
    requires: &[],
    // Who sits in which room is a project's file, not a person's settings.
    config: None,
};

/// How this plugin identifies itself when it opens a session.
pub(crate) fn identity() -> ClientIdentity {
    ClientIdentity {
        name: "rooms".into(),
        surface: SURFACE.into(),
    }
}

/// Registers `/room`, the tool an agent opens a room with, and the hook that
/// seats and fans out. Each is handed the host by the kernel at the call, so
/// this plugin keeps none.
#[derive(Debug, Default)]
pub struct RoomsPlugin {
    writer: Arc<RoomWriter>,
    hook: Arc<RoomsHook>,
    recovery: recovery::Recovery,
}

#[async_trait]
impl Plugin for RoomsPlugin {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }

    fn register(&self, registrar: &mut Registrar) -> Result<(), PluginError> {
        registrar.add(Contribution::Command(
            Arc::new(RoomCommand) as Arc<dyn Command>
        ));
        registrar.add(Contribution::Hook(self.hook.clone() as Arc<dyn Hook>));
        registrar.add(Contribution::Tool(Arc::new(OpenRoomTool) as Arc<dyn Tool>));
        registrar.add(Contribution::Tool(Arc::new(SeatTool) as Arc<dyn Tool>));
        registrar.add(Contribution::Tool(Arc::new(UnseatTool) as Arc<dyn Tool>));
        registrar.add(Contribution::Tool(Arc::new(CloseRoomTool) as Arc<dyn Tool>));
        registrar.add(Contribution::Tool(Arc::new(ListenTool) as Arc<dyn Tool>));
        registrar.add(Contribution::Context(
            Arc::new(Reader) as Arc<dyn ContextContributor>
        ));
        let writer: Arc<dyn bingo_services::room_post::RoomPost> = self.writer.clone();
        registrar.add(Contribution::service(
            &bingo_services::room_post::KEY,
            writer.clone(),
            Some(bingo_services::room_post::wire(writer)),
        ));
        let control: Arc<dyn bingo_services::rooms_control::RoomsControl> = self.writer.clone();
        registrar.add(Contribution::service(
            &bingo_services::rooms_control::KEY,
            control.clone(),
            Some(bingo_services::rooms_control::wire(control)),
        ));
        registrar.add(guide::contribution(registrar));
        Ok(())
    }
    async fn start(&self, host: bingo_sdk::HostHandle) -> Result<(), PluginError> {
        self.writer.start(host.clone());
        self.recovery.start(self.hook.clone(), host);
        Ok(())
    }

    async fn stop(&self) -> Result<(), PluginError> {
        self.recovery.stop().await;
        self.hook.stop();
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests;

#[cfg(test)]
mod plugin_tests {
    use super::*;
    use bingo_sdk::Env;

    #[test]
    fn the_manifest_says_what_it_provides_and_claims_no_settings() {
        assert_eq!(MANIFEST.id, "bingo.rooms");
        assert_eq!(
            MANIFEST.provides,
            [
                "command:room",
                "hook:rooms",
                "tool:OpenRoom",
                "tool:Seat",
                "tool:Unseat",
                "tool:CloseRoom",
                "tool:Listen",
                "context:rooms",
                bingo_services::room_post::CAPABILITY,
                bingo_services::rooms_control::CAPABILITY,
                &format!("service:{}", bingo_sdk::Pages::key(MANIFEST.id).name()),
            ]
        );
        assert!(MANIFEST.requires.is_empty());
        assert!(MANIFEST.config.is_none());
    }

    #[test]
    fn registering_reads_nothing_and_contributes_what_the_manifest_promises() {
        let mut registrar = Registrar::new(
            PLUGIN,
            serde_json::Value::Null,
            Env::rooted("/nowhere"),
            Default::default(),
        );
        RoomsPlugin::default()
            .register(&mut registrar)
            .expect("registration does no I/O");
        let contributions = registrar.into_contributions();
        assert_eq!(contributions.len(), MANIFEST.provides.len());
        assert!(matches!(&contributions[0], Contribution::Command(c) if c.spec().name == "room"));
        assert!(matches!(&contributions[1], Contribution::Hook(h) if h.id() == "rooms"));
        assert!(matches!(&contributions[2], Contribution::Tool(t) if t.spec().name == "OpenRoom"));
        assert!(matches!(&contributions[3], Contribution::Tool(t) if t.spec().name == "Seat"));
        assert!(matches!(&contributions[4], Contribution::Tool(t) if t.spec().name == "Unseat"));
        assert!(matches!(&contributions[5], Contribution::Tool(t) if t.spec().name == "CloseRoom"));
        assert!(matches!(&contributions[6], Contribution::Tool(t) if t.spec().name == "Listen"));
        assert!(matches!(&contributions[7], Contribution::Context(c) if c.id() == "rooms"));
        match &contributions[10] {
            Contribution::Service { key, wire, .. } => {
                assert_eq!(key, bingo_sdk::Pages::key(MANIFEST.id).name());
                assert!(wire.is_none(), "a page is read in process");
            }
            other => panic!("expected the page service, got {other:?}"),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_second_plugin_rearms_persisted_debt_and_patience_once() {
        let fleet = crate::tests::Fleet::default();
        let root = fleet.root();
        fleet.child(&root, "scout");
        let first = RoomsPlugin::default();
        first.start(fleet.handle()).await.unwrap();
        let id = seat(
            &fleet.handle(),
            &root,
            std::path::Path::new("/work"),
            Opening::person("design", None),
            &[Seat::named("scout")],
        )
        .await
        .unwrap();
        fleet.post(
            &id,
            "@scout inspect the build",
            None,
            jiff::Timestamp::now() - jiff::SignedDuration::from_secs(3600),
        );
        first.stop().await.unwrap();
        drop(first);
        let second = RoomsPlugin::default();
        second.start(fleet.handle()).await.unwrap();
        for _ in 0..16 {
            tokio::task::yield_now().await;
        }
        assert_eq!(
            fleet.delivered().len(),
            2,
            "one debt chase and one patient-seat nudge"
        );
        tokio::time::advance(std::time::Duration::from_secs(3600)).await;
        second.start(fleet.handle()).await.unwrap();
        for _ in 0..16 {
            tokio::task::yield_now().await;
        }
        assert_eq!(
            fleet.delivered().len(),
            2,
            "reconciliation does not duplicate overdue deliveries"
        );
        second.stop().await.unwrap();
    }
}

#[cfg(test)]
mod control_tests;
