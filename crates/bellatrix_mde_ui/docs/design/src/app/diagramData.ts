import type { DiagramFamily } from './types';
export const families: { id:DiagramFamily; name:string; shorthand:string; description:string; }[] = [
  {id:'uml',name:'UML 2.5',shorthand:'UML',description:'Software design & behavior'},
  {id:'c4',name:'C4 Model',shorthand:'C4',description:'Architecture at four levels'},
  {id:'mindmap',name:'Mind Map',shorthand:'MIND',description:'Topics and relationships'},
  {id:'flowchart',name:'Flowchart',shorthand:'FLOW',description:'Processes and decisions'}
];
export const diagrams: { family:DiagramFamily; label:string; type:string; name:string; }[] = [
  {family:'uml',label:'Domain model',type:'Class diagram',name:'Domain model'},
  {family:'c4',label:'System architecture',type:'Container diagram',name:'System architecture'},
  {family:'mindmap',label:'Product discovery',type:'Mind map',name:'Product discovery'},
  {family:'flowchart',label:'Order processing',type:'Flowchart',name:'Order processing'}
];
export const elements:Record<DiagramFamily,{name:string;kind:string;namespace:string;notes:string}> = {
  uml:{name:'OrderService',kind:'Class',namespace:'commerce.domain',notes:'Coordinates order creation and validation.'},
  c4:{name:'Order API',kind:'Container',namespace:'Commerce platform',notes:'HTTP API for order management.'},
  mindmap:{name:'Product strategy',kind:'Central topic',namespace:'Product discovery',notes:'Planning branches and research topics.'},
  flowchart:{name:'Validate input',kind:'Decision',namespace:'Order processing',notes:'Checks whether the request is valid.'}
};
// Scope index for the library view, independent of editor behavior.
export const diagramTypes:Record<DiagramFamily,string[]> = {
  uml:['Class','Package','Object','Composite structure','Component','Deployment','Use case',
       'Sequence','Communication','Timing','Interaction overview','State machine','Activity',
       'Information flow','Profile'],
  c4:['System landscape','System context (L1)','Container (L2)','Component (L3)',
      'Code (L4)','Dynamic','Deployment'],
  mindmap:['Radial mind map','Tree mind map','Concept map'],
  flowchart:['Basic flowchart','Cross-functional flowchart','Swimlane flowchart']
};
