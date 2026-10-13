export function register(on) {
  on('tool.call', {tool:'Read'}, async ($) => {
    const fs = await $.fs.read({path:'world.txt'});
    const env = await $.env.get({name:'BINGO_WORLD_PARITY'});
    const process = await $.process.run({argv:['node','-e',"process.stdout.write('child')"]});
    const url = await $.env.get({name:'BINGO_WORLD_URL'});
    const http = await $.http.fetch({url});
    const agent = await $.agent.list();
    let mcp;
    try { await $.mcp.connect({server:'not-configured'}); }
    catch(error) { mcp = String(error).includes('not configured'); }
    return {parts:[{type:'text',text:JSON.stringify({fs,env,process:process.stdout,http:http.body,agent:agent.length,mcp})}],isError:false};
  });
}
