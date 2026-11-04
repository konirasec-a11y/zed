use crate::document::{Documentation, ElementPage};

pub const CSS: &str = r#"
:root{color-scheme:light dark;font:14px/1.55 system-ui,Segoe UI,Arial,sans-serif}
*{box-sizing:border-box}body{margin:0;background:light-dark(#fff,#1d1f23);color:light-dark(#22252b,#e9eaed)}
header{padding:13px 24px;border-bottom:1px solid light-dark(#e0e2e6,#35383e)}
.layout{display:grid;grid-template-columns:248px minmax(0,1fr);min-height:calc(100vh - 53px)}
nav{padding:16px;border-right:1px solid light-dark(#e0e2e6,#35383e);overflow-wrap:anywhere}
nav h3{font-size:11px;letter-spacing:1px;text-transform:uppercase;color:light-dark(#6b7280,#9ca3af);margin:22px 0 8px}
nav a{display:block;padding:3px 6px;border-radius:4px}nav a:hover{background:light-dark(#f1f3f5,#30343a)}
main{padding:32px 38px;max-width:1180px;width:100%;min-width:0}
a{color:light-dark(#185c9a,#91bdf1);text-decoration:none}a:hover{text-decoration:underline}
h1,h2{line-height:1.25}h1{font-size:25px;font-weight:650}h2{font-size:17px;margin-top:28px}
h3{font-size:14px}small,.muted{opacity:.73}code,pre{font-family:ui-monospace,SFMono-Regular,Consolas,monospace}
pre{white-space:pre-wrap;overflow-wrap:anywhere;background:light-dark(#f5f6f8,#25282d);border:1px solid light-dark(#e0e2e6,#383b41);border-radius:5px;padding:12px}
figure{margin:22px 0}figure img{display:block;width:100%;height:auto;border:1px solid #8884;border-radius:4px}
.table{width:100%;border-collapse:collapse}.table td,.table th{text-align:left;vertical-align:top;border-bottom:1px solid #8883;padding:7px}
.kpis{display:flex;flex-wrap:wrap;gap:12px}.kpis div{border:1px solid #8884;padding:12px 18px;border-radius:5px}
.tags{display:flex;flex-wrap:wrap;gap:5px}.tag{font-size:12px;padding:2px 6px;border:1px solid #8884;border-radius:4px}
@media(max-width:760px){.layout{display:block}nav{border-right:0;border-bottom:1px solid #8884}main{padding:18px}}
"#;

pub fn esc(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
/// Hex encoding is injective, unlike replacing punctuation with underscores.
pub fn safe_id(id: &str) -> String {
    let mut output = String::from("id-");
    for byte in id.bytes() {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}
fn nav(project: &Documentation, prefix: &str) -> String {
    let mut links = format!("<a href=\"{prefix}index.html\">Overview</a>");
    links.push_str("<h3>Diagrams</h3>");
    for diagram in &project.diagrams {
        links.push_str(&format!(
            "<a href=\"{prefix}index.html#diagram-{}\">{}</a>",
            safe_id(&diagram.id),
            esc(&diagram.name)
        ));
    }
    links.push_str("<h3>Elements</h3>");
    for element in &project.elements {
        links.push_str(&format!(
            "<a href=\"{prefix}elements/{}.html\" title=\"{}\">{}</a>",
            safe_id(&element.id),
            esc(&element.kind),
            esc(&element.label)
        ));
    }
    links
}
fn shell(project: &Documentation, title: &str, content: &str, prefix: &str) -> String {
    format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>{} · {}</title><link rel="stylesheet" href="{}assets/style.css"></head>
<body><header><strong>Bellatrix</strong> <span class="muted">/ {}</span></header>
<div class="layout"><nav aria-label="Project documentation">{}</nav>
<main>{}</main></div></body></html>"#,
        esc(title),
        esc(&project.name),
        prefix,
        esc(&project.name),
        nav(project, prefix),
        content
    )
}
pub fn index(project: &Documentation) -> String {
    let mut body = format!(
        "<h1>{}</h1><p class=muted>Technical bellatrix-project documentation · Model snapshot</p>",
        esc(&project.name)
    );
    body.push_str(&format!("<div class=kpis><div><b>{}</b> elements</div><div><b>{}</b> relations</div><div><b>{}</b> diagrams</div></div>",
        project.elements.len(), project.relations.len(), project.diagrams.len()));
    body.push_str("<h2>Model elements</h2><table class=table><thead><tr><th>Name</th><th>Type</th><th>Description</th></tr></thead><tbody>");
    for element in &project.elements {
        body.push_str(&format!(
            "<tr><td><a href=\"elements/{}.html\">{}</a></td><td>{}</td><td>{}</td></tr>",
            safe_id(&element.id),
            esc(&element.label),
            esc(&element.kind),
            esc(element.description.as_deref().unwrap_or(""))
        ));
    }
    body.push_str("</tbody></table>");
    body.push_str(&relations_index(project));
    body.push_str("<h2>Diagrams</h2>");
    for diagram in &project.diagrams {
        body.push_str(&format!("<figure id=\"diagram-{}\"><figcaption><b>{}</b></figcaption><img loading=\"lazy\" alt=\"{}\" src=\"diagrams/{}.svg\"></figure>",
            safe_id(&diagram.id), esc(&diagram.name), esc(&diagram.name), safe_id(&diagram.id)));
    }
    shell(project, &project.name, &body, "")
}
fn relations_index(project: &Documentation) -> String {
    if project.relations.is_empty() {
        return String::new();
    }
    let mut body = String::from(
        "<h2>Relationships</h2><table class=table><thead><tr><th>Type</th><th>From</th><th>To</th></tr></thead><tbody>",
    );
    let mut relations = project.relations.iter().collect::<Vec<_>>();
    relations.sort_by(|left, right| {
        (&left.kind, &left.from, &left.to, &left.id).cmp(&(
            &right.kind,
            &right.from,
            &right.to,
            &right.id,
        ))
    });
    for relation in relations {
        let linked = |id: &str| {
            let name = project
                .element(id)
                .map(|element| element.label.as_str())
                .unwrap_or(id);
            format!(
                "<a href=\"elements/{}.html\">{}</a>",
                safe_id(id),
                esc(name)
            )
        };
        body.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
            esc(&relation.kind),
            linked(&relation.from),
            linked(&relation.to)
        ));
    }
    body.push_str("</tbody></table>");
    body
}
fn relation_rows(project: &Documentation, element: &ElementPage) -> String {
    let mut body = String::from("<h2>Relationships</h2><table class=table><tbody>");
    for relation in project
        .relations
        .iter()
        .filter(|relation| relation.from == element.id || relation.to == element.id)
    {
        let (other, direction) = if relation.from == element.id {
            (&relation.to, "outgoing")
        } else {
            (&relation.from, "incoming")
        };
        let label = project
            .element(other)
            .map(|element| element.name.as_str())
            .unwrap_or(other);
        body.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td><a href=\"{}.html\">{}</a></td></tr>",
            esc(&relation.kind),
            direction,
            safe_id(other),
            esc(label)
        ));
    }
    body.push_str("</tbody></table>");
    body
}
pub fn element_page(project: &Documentation, element: &ElementPage) -> String {
    let label = &element.label;
    let mut body = format!(
        "<h1>{}</h1><p class=muted>{} · {}</p>",
        esc(label),
        esc(&element.kind),
        esc(&element.id)
    );
    if let Some(description) = &element.description {
        body.push_str(&format!(
            "<h2>Description</h2><pre>{}</pre>",
            esc(description)
        ));
    }
    if !element.properties.is_empty() {
        body.push_str("<h2>Properties</h2><table class=table><tbody>");
        for (key, value) in &element.properties {
            body.push_str(&format!(
                "<tr><th>{}</th><td><code>{}</code></td></tr>",
                esc(key),
                esc(value)
            ));
        }
        body.push_str("</tbody></table>");
    }
    if !element.annotations.is_empty() {
        body.push_str("<h2>Annotations</h2>");
        for annotation in &element.annotations {
            body.push_str(&format!(
                "<h3>{}</h3><pre>{}</pre>",
                esc(&annotation.language),
                esc(&annotation.source)
            ));
        }
    }
    if !element.implementations.is_empty() {
        body.push_str("<h2>Implementations</h2>");
        for implementation in &element.implementations {
            body.push_str(&format!(
                "<h3>{}</h3><pre><code>{}</code></pre>",
                esc(&implementation.language),
                esc(&implementation.source)
            ));
        }
    }
    let diagrams = project
        .diagrams
        .iter()
        .filter(|diagram| {
            diagram
                .page_subjects
                .iter()
                .any(|subject| subject == &element.id)
        })
        .collect::<Vec<_>>();
    if !diagrams.is_empty() {
        body.push_str("<h2>Appears in</h2><div class=tags>");
        for diagram in diagrams {
            body.push_str(&format!(
                "<a class=tag href=\"../index.html#diagram-{}\">{}</a>",
                safe_id(&diagram.id),
                esc(&diagram.name)
            ));
        }
        body.push_str("</div>");
    }
    body.push_str(&relation_rows(project, element));
    shell(project, label, &body, "../")
}
