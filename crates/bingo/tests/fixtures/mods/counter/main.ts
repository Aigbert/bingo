export function register(on) {
  on('session.start', async $ => {
    await $.command.register({name:'counter', hint:'Open counter', aliases:[], args:{kind:'none'}, instant:true, family:'ui'}, async api => {
      await api.ui.open({id:'counter',title:'Counter',focus:true,closeOnEscape:true,holdToasts:true});
      return {kind:'applied'};
    });
  });
  on('ui.render', {component:'Pane',requestId:'counter'}, async ($, event) => {
    const count = (await $.state.get('count'))?.value ?? 0;
    const name = (await $.state.get('name'))?.value ?? '';
    return {kind:'Box',flexDirection:'column',children:[
      {kind:'Text',children:[`Count: ${count}`]},
      {kind:'Button',label:'Increment',hotkey:'i',onPress:async () => { await $.state.set('count',count+1); }},
      {kind:'Input',key:'name',label:'Name',value:name,onInput:async value => { await $.state.set('name',value); }},
    ]};
  });
}
