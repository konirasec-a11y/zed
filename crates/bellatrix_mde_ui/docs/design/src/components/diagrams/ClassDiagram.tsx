import * as React from 'react';
import { ClassNode, Relation } from './SvgNodes';
export function ClassDiagram(){return <svg className="specimen-diagram" viewBox="0 0 1060 650" role="img" aria-label="UML class diagram, illustrative example of the ordering domain">
 <defs><marker id="class-open" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="8" markerHeight="8" orient="auto-start-reverse"><path d="M1 1 9 5 1 9" fill="none" stroke="var(--diagram-edge)" strokeWidth="1.1"/></marker><marker id="class-triangle" viewBox="0 0 12 12" refX="11" refY="6" markerWidth="10" markerHeight="10" orient="auto"><path d="M1 1 11 6 1 11z" fill="var(--paper-white)" stroke="var(--diagram-edge)" strokeWidth="1"/></marker></defs>

 <g className="relationship-layer">
  <Relation d="M331 191 H438" label="creates" labelX={365} labelY={182}/>
  <Relation d="M562 287 V381" label="1..*" labelX={574} labelY={340}/>
  <Relation d="M772 214 H642" label="places" labelX={694} labelY={204}/>
  <Relation d="M655 457 H768" label="refers to" labelX={682} labelY={446}/>
  <path d="M220 268V372" stroke="var(--diagram-edge)" strokeWidth="1.6" strokeDasharray="7 5" markerEnd="url(#class-triangle)" fill="none"/>
  <path d="M562 294l-9-13 9-13 9 13z" fill="var(--paper-white)" stroke="var(--diagram-edge)" strokeWidth="1.6" transform="translate(0 6)"/>
  <text className="relation-label" x="244" y="337">implements</text>
  <text className="relation-small" x="433" y="184">1</text><text className="relation-small" x="644" y="184">*</text>
 </g>
 <ClassNode x={110} y={135} w={221} title="OrderService" attributes={['− repo: OrderRepository','− validator: Validator']} operations={['+ createOrder(): Order']} selected/>
 <ClassNode x={438} y={135} w={204} title="Order" attributes={['+ id: UUID','+ status: OrderStatus','+ total: Decimal']} operations={['+ addItem(item): void']}/>
 <ClassNode x={772} y={135} w={184} title="Customer" attributes={['+ id: UUID','+ email: String']} operations={['+ placeOrder(): void']}/>
 <ClassNode x={110} y={372} w={221} title="OrderRepository" stereotype="interface" attributes={['+ findById(id): Order']} operations={['+ save(order): void']}/>
 <ClassNode x={460} y={381} w={195} title="OrderItem" attributes={['+ quantity: Integer','+ price: Decimal']} operations={['+ subtotal(): Decimal']}/>
 <ClassNode x={768} y={381} w={184} title="Product" attributes={['+ sku: String','+ price: Decimal']} operations={['+ isActive(): bool']}/>
 <g className="canvas-selection-toolbar" transform="translate(110 101)"><rect width="154" height="27" rx="4" className="mini-toolbar-bg"/><path d="M31 6v15M62 6v15M93 6v15M124 6v15" className="mini-toolbar-divider"/><text x="14" y="18">+</text><text x="45" y="18">◇</text><text x="76" y="18">▹</text><text x="107" y="18">⋯</text><text x="137" y="18">↗</text></g>
 </svg>}
