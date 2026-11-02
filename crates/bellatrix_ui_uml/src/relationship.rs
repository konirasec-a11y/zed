//! UML relationship notation. SVG is generated from a persistent path View;
//! no semantic changes, routing rules or Cordis dependencies live in this crate.
use bellatrix_design_system::Palette;
use gpui::{App, IntoElement, RenderOnce, Window, div, prelude::*, svg};

/// Coordinates are local to the SVG element; host computes zoom/pan.
#[derive(IntoElement)]
pub struct UmlRelationGlyph {
    kind:String,
    width:f32,
    height:f32,
    from:(f32,f32),
    to:(f32,f32),
    cubic:Option<((f32,f32),(f32,f32))>,
    selected:bool,
}
impl UmlRelationGlyph {
    pub fn new(kind:impl Into<String>,width:f32,height:f32,from:(f32,f32),to:(f32,f32))->Self {
        Self {kind:kind.into(),width:width.max(1.0),height:height.max(1.0),from,to,cubic:None,selected:false}
    }
    pub fn cubic(mut self, a:(f32,f32), b:(f32,f32))->Self {
        self.cubic=Some((a,b));self
    }
    pub fn selected(mut self, value:bool)->Self {self.selected=value;self}
}
fn marker_paths(kind:&str, from:(f32,f32),to:(f32,f32),stroke:&str,background:&str,
                cubic:Option<((f32,f32),(f32,f32))>)->String {
    let (sx,sy)=from;
    let (tx,ty)=to;
    let (dx,dy)=if let Some((_,control))=cubic {
        (tx-control.0,ty-control.1)
    } else {(tx-sx,ty-sy)};
    let len=(dx*dx+dy*dy).sqrt().max(0.01);
    let (ux,uy,nx,ny)=(dx/len,dy/len,-dy/len,dx/len);
    let start_unit=if let Some((control,_))=cubic {
        let (dx,dy)=(control.0-sx,control.1-sy);
        let len=(dx*dx+dy*dy).sqrt().max(0.01);
        (dx/len,dy/len)
    } else {(ux,uy)};
    let (su,sv)=start_unit;
    let (snx,sny)=(-sv,su);
    let triangle=|filled:bool|format!(
        "<polygon points='{tx:.2},{ty:.2} {:.2},{:.2} {:.2},{:.2}' fill='{}' stroke='{stroke}' stroke-width='1.6'/>",
        tx-ux*16.0+nx*7.0,ty-uy*16.0+ny*7.0,
        tx-ux*16.0-nx*7.0,ty-uy*16.0-ny*7.0,
        if filled {stroke}else{background});
    let open_arrow=||format!(
        "<path d='M {:.2} {:.2} L {tx:.2} {ty:.2} L {:.2} {:.2}' fill='none' stroke='{stroke}' stroke-width='1.6'/>",
        tx-ux*11.0+nx*6.0,ty-uy*11.0+ny*6.0,
        tx-ux*11.0-nx*6.0,ty-uy*11.0-ny*6.0);
    let diamond=|filled:bool|format!(
        "<polygon points='{sx:.2},{sy:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}' fill='{}' stroke='{stroke}' stroke-width='1.6'/>",
        sx+su*10.0+snx*6.0,sy+sv*10.0+sny*6.0,
        sx+su*20.0,sy+sv*20.0,
        sx+su*10.0-snx*6.0,sy+sv*10.0-sny*6.0,
        if filled {stroke}else{background});
    match kind {
        "uml.generalization" | "uml.interface_realization" | "uml.realization" => triangle(false),
        "uml.directed_association" | "uml.dependency" | "uml.template_binding" => open_arrow(),
        "uml.aggregation" => diamond(false),
        "uml.composition" => diamond(true),
        "uml.containment" => format!(
            "<circle cx='{sx:.2}' cy='{sy:.2}' r='7' fill='{background}' stroke='{stroke}' stroke-width='1.4'/>             <path d='M {:.2} {sy:.2} H {:.2} M {sx:.2} {:.2} V {:.2}' stroke='{stroke}' stroke-width='1.2'/>",
            sx-4.0,sx+4.0,sy-4.0,sy+4.0),
        _ => String::new(),
    }
}
fn dash(kind:&str)->&'static str {
    if matches!(kind,"uml.dependency"|"uml.interface_realization"|"uml.template_binding"|"uml.realization"|"uml.note_link"|"uml.constraint_link") {
        " stroke-dasharray='6 5'"
    }else{""}
}
impl RenderOnce for UmlRelationGlyph {
    fn render(self,_:&mut Window,cx:&mut App)->impl IntoElement {
        let palette=Palette::current(cx);
        let stroke=format!("#{:06X}",if self.selected {palette.selection}else{palette.text});
        let background=format!("#{:06X}",palette.canvas);
        let (x1,y1)=self.from;let (x2,y2)=self.to;
        let d=if let Some(((a,b),(c,d)))=self.cubic {
            format!("M {x1:.2} {y1:.2} C {a:.2} {b:.2} {c:.2} {d:.2} {x2:.2} {y2:.2}")
        }else{format!("M {x1:.2} {y1:.2} L {x2:.2} {y2:.2}")};
        let marks=marker_paths(&self.kind,self.from,self.to,&stroke,&background,self.cubic);
        let content=format!(
            "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 {:.2} {:.2}' preserveAspectRatio='none'>             <path d='{d}' fill='none' stroke='{stroke}' stroke-width='1.6'{} />{marks}</svg>",
            self.width,self.height,dash(&self.kind));
        // The caller sets the local SVG bounds; `viewBox` is normalized here.
        div().size_full().child(svg().data(content.as_bytes()).size_full())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn associations_have_different_endpoint_notation() {
        assert!(marker_paths("uml.association",(0.,0.),(30.,0.),"#000","#fff",None).is_empty());
        assert!(marker_paths("uml.generalization",(0.,0.),(30.,0.),"#000","#fff",None).contains("polygon"));
        assert!(marker_paths("uml.composition",(0.,0.),(30.,0.),"#000","#fff",None).contains("polygon"));
        assert!(marker_paths("uml.dependency",(0.,0.),(30.,0.),"#000","#fff",None).contains("path"));
        assert_eq!(dash("uml.interface_realization")," stroke-dasharray='6 5'");
    }
}
