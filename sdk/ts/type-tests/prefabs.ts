import type { Command, Project, Prefab, PrefabLink } from '../src/generated';
const prototype = { id:'prototype', name:'Object', root:'node', nodes:{ node:{name:'Object',components:{}} } } satisfies Prefab;
const commands: Command[] = [
  {op:'create_prefab',scene_id:'scene',root_entity_id:'entity',prefab_id:'prototype',name:'Object',nodes:{entity:'node'}},
  {op:'instantiate_prefab',source:'prototype',scene_id:'scene',parent:null,entities:{node:'instance'}},
  {op:'update_prefab',prefab:prototype},
  {op:'revert_prefab_component',scene_id:'scene',entity_id:'instance',component:'Transform'},
  {op:'detach_prefab',scene_id:'scene',root_entity_id:'instance'},
  {op:'delete_prefab_instance',scene_id:'scene',root_entity_id:'instance'},
  {op:'delete_prefab',prefab_id:'prototype'},
];
const link = {source:'prototype',root:'instance',node:'node',overrides:['Transform']} satisfies PrefabLink;
declare const legacy: Project;
const sources: Record<string, Prefab> = legacy.prefabs ?? {};
// @ts-expect-error Instantiation must supply the full explicit identity map.
const missing = {op:'instantiate_prefab',source:'prototype',scene_id:'scene'} satisfies Command;
// @ts-expect-error Overrides name components; JSON patch paths are not accepted.
const patch = {source:'prototype',root:'instance',node:'node',overrides:{'/Transform/x':2}} satisfies PrefabLink;
// @ts-expect-error A prototype update carries a typed prototype rather than opaque JSON.
const untyped = {op:'update_prefab',prefab:3} satisfies Command;
void [commands,link,sources,missing,patch,untyped];
