export function register(on) {
  on('session.start', async $ => {
    await $.command.register({name:'chat-ready', hint:'Check the chat fixture', aliases:[], args:{kind:'none'}, instant:true, family:'ui'},
      async () => ({kind:'applied', message:'Chat ready'}));
  });
  on('ui.render', {component:'AssistantMessage', surface:'chat'}, async ($, event) => {
    const {Box, Text, Button} = await $.ui.resolve(event);
    const count = (await $.state.get('count'))?.value ?? 0;
    return {kind:Box, children:[
      {kind:Text, children:[`Chat count: ${count}`]},
      {kind:Button, children:['Increment'], onPress:async () => {
        await $.state.set('count', count + 1);
      }},
    ]};
  });
}
