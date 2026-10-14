export const register = on => {
  on('ui.render', {component: 'Compaction'}, async ($, e) => {
    const {Text = 'Text'} = await $.ui.resolve(e);
    return {kind: Text, children: [`Compacted ${e.props.before} to ${e.props.after} on ${e.surface}`]};
  });
  on('ui.render', {component: 'Reasoning'}, async ($, e, next) => {
    const {Box = 'Box', Text = 'Text'} = await $.ui.resolve(e);
    return {kind: Box, children: [
      {kind: Text, children: ['Thinking follows']},
      await next({...e, props: {...e.props, text: 'revised thought'}}),
    ]};
  });
};
