export function register(on) {
  on('session.start', async $ => {
    await $.ui.open({id:'art',title:'Art',focus:false,closeOnEscape:true,
      place:[{rect:{anchor:'bottomRight',width:22,height:12,x:1,y:1}}],
      size:{rows:{min:2,ideal:12,max:12},columns:{min:8,ideal:22,max:22}}});
  });
  for (const hook of ['turn.start', 'turn.complete']) {
    on(hook, async ($, event, next) => {
      const count = (await $.state.get('turns'))?.value ?? 0;
      await $.state.set('turns', count + 1);
      return next(event);
    });
  }
  on('ui.render', {component:'Pane',requestId:'art'}, async $ => {
    const count = (await $.state.get('turns'))?.value ?? 0;
    return {kind:'Box',flexDirection:'column',children:[
      {kind:'Text',children:[' /\\_/\\\n( o.o )\n > ^ <']},
      {kind:'Text',children:[`ART FRAME ${count}`]},
    ]};
  });
}
