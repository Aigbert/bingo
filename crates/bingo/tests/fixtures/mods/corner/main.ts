export function register(on) {
  on('session.start', async $ => {
    await $.command.register({name:'corner',hint:'Open corner',aliases:[],args:{kind:'none'},instant:true,family:'ui'}, async api => {
      await api.ui.open({id:'corner',title:'Corner',focus:true,closeOnEscape:true,
        place:[{rect:{anchor:'bottomRight',width:'40%',height:'30%'}}],
        size:{rows:{min:2,ideal:8,max:'100%'},columns:{min:8,ideal:40,max:'100%'}}});
      return {kind:'applied'};
    });
  });
  on('ui.render', {component:'Pane',requestId:'corner'}, async ($, event) => {
    const {columns, rows} = event.viewport;
    return {kind:'Raster',columns,rows,cells:Array.from({length:rows}, () => '░'.repeat(columns))};
  });
}
