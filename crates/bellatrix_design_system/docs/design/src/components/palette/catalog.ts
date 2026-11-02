import type { DiagramFamily, PaletteGroup, ShapeKind } from '../../app/types';
// Presentation catalog. A label is vocabulary, not a schema or serialized UML metamodel.
const group = (id:string,title:string, rows: [string,ShapeKind][]):PaletteGroup => ({
  id,title,items:rows.map(([label,kind],index)=>({id:`${id}-${index}`,label,kind}))
});
const u = {
  classes:group('uml-classes','Classes & structure',[
    ['Class','class'],['Interface','interface'],['Enumeration','enum'],['Primitive type','class'],
    ['Data type','class'],['Signal','class'],['Association class','class'],['Template','class'],
    ['Package','package'],['Model','package'],['Subsystem','package'],['Comment','note']]),
  relationships:group('uml-relationships','Relationships',[
    ['Association','association'],['Directed association','association'],['Aggregation','aggregate'],
    ['Composition','composition'],['Dependency','dependency'],['Generalization','inheritance'],
    ['Interface realization','inheritance'],['Abstraction','dependency'],['Usage','dependency'],
    ['Containment','association'],['Information flow','association']]),
  usecase:group('uml-usecase','Use cases',[
    ['Actor','actor'],['Use case','usecase'],['Subject boundary','boundary'],['Include','dependency'],
    ['Extend','dependency'],['Extension point','note'],['Generalization','inheritance']]),
  sequence:group('uml-sequence','Sequence & interaction',[
    ['Lifeline','lifeline'],['Actor lifeline','actor'],['Message','message'],['Async message','message'],
    ['Reply message','message'],['Create message','message'],['Delete message','message'],
    ['Found message','message'],['Lost message','message'],['Self message','message'],
    ['Execution','lifeline'],['Combined fragment','boundary'],['Interaction use','boundary'],
    ['State invariant','state'],['Gate','connector'],['Duration constraint','note']]),
  activity:group('uml-activity','Activities',[
    ['Initial node','state'],['Action','activity'],['Activity','activity'],['Decision','decision'],
    ['Merge','decision'],['Fork','fork'],['Join','fork'],['Activity final','state'],
    ['Flow final','state'],['Object node','activity'],['Input pin','connector'],
    ['Output pin','connector'],['Control flow','relation'],['Object flow','relation'],
    ['Partition','boundary'],['Expansion region','boundary'],['Accept event','activity']]),
  state:group('uml-state','State machines',[
    ['State','state'],['Initial pseudostate','state'],['Final state','state'],
    ['Choice','decision'],['Junction','decision'],['Fork / join','fork'],['History','state'],
    ['Entry point','connector'],['Exit point','connector'],['Transition','relation'],
    ['Region','boundary'],['Terminate','state']]),
  components:group('uml-components','Components & deployment',[
    ['Component','component'],['Artifact','document'],['Port','connector'],['Part','class'],
    ['Provided interface','interface'],['Required interface','interface'],['Connector','relation'],
    ['Node','deployment'],['Device','deployment'],['Execution environment','deployment'],
    ['Deployment specification','document'],['Communication path','association'],
    ['Deployment','dependency'],['Manifestation','dependency']]),
  others:group('uml-others','Other UML diagrams',[
    ['Instance specification','class'],['Slot','class'],['Object','class'],
    ['Collaboration','component'],['Interaction overview','boundary'],['Timing frame','boundary'],
    ['Profile','package'],['Stereotype','class'],['Metaclass','class'],['Extension','relation'],
    ['Constraint','note'],['Time observation','note'],['Duration observation','note']])
};
const c4 = {
  context:group('c4-context','System context',[
    ['Person','person'],['External person','person'],['Software system','system'],
    ['External system','external'],['Enterprise boundary','boundary'],['System boundary','boundary'],
    ['Relationship','relation'],['Bidirectional relationship','relation']]),
  container:group('c4-containers','Container view',[
    ['Container / application','container'],['Web application','container'],['Mobile application','container'],
    ['Microservice','container'],['API','component'],['Message broker','container'],
    ['Database','database'],['Data store','database'],['Container boundary','boundary']]),
  component:group('c4-components','Component view',[
    ['Component','component'],['External component','external'],['Component boundary','boundary'],
    ['Dependency','relation'],['Asynchronous relationship','relation']]),
  supporting:group('c4-support','Supporting views',[
    ['Deployment node','deployment'],['Infrastructure node','deployment'],
    ['Deployment environment','boundary'],['Container instance','container'],
    ['Dynamic step','message'],['System landscape','system'],
    ['Code element','class'],['Technology label','note'],['Description','note']])
};
const mindmap = {
  topics:group('mindmap-topics','Topics',[
    ['Central topic','topic'],['Main topic','branch'],['Subtopic','branch'],
    ['Sibling topic','branch'],['Floating topic','topic'],['Callout','callout']]),
  structure:group('mindmap-structure','Structure',[
    ['Branch connector','relation'],['Relationship','relation'],['Boundary','boundary'],
    ['Summary','note'],['Grouping','boundary'],['Cross-link','relation']]),
  markers:group('mindmap-markers','Annotations & markers',[
    ['Priority marker','state'],['Progress marker','state'],['Flag','note'],
    ['Note','note'],['Link','relation'],['Tag','callout'],['Image','document']])
};
const flowchart = {
  basic:group('flowchart-basic','Common symbols',[
    ['Start / end','terminator'],['Process','process'],['Decision','decision'],
    ['Input / output','input'],['Document','document'],['Predefined process','predefined'],
    ['Database / storage','database'],['Flow line','relation'],['Connector','connector']]),
  advanced:group('flowchart-advanced','Extended symbols',[
    ['Manual input','input'],['Manual operation','process'],['Preparation','process'],
    ['Delay','terminator'],['Display','document'],['Multiple documents','document'],
    ['Stored data','database'],['Internal storage','database'],['Direct access storage','database'],
    ['Sequential access storage','database'],['Off-page connector','connector'],
    ['Annotation','note'],['Subprocess','predefined']]),
  lanes:group('flowchart-lanes','Layout & connectors',[
    ['Swimlane','boundary'],['Pool','boundary'],['Group','boundary'],
    ['Yes / no branch','relation'],['Arrow connector','relation'],['Dotted connector','relation']])
};
export const catalog: Record<DiagramFamily,PaletteGroup[]> = {
  uml:Object.values(u),c4:Object.values(c4),mindmap:Object.values(mindmap),flowchart:Object.values(flowchart)
};
export const familyTotals = Object.fromEntries(Object.entries(catalog).map(([id,groups])=>[
  id,groups.reduce((total,g)=>total+g.items.length,0)
])) as Record<DiagramFamily,number>;
