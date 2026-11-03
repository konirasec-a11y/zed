//! Target-neutral M2T rendering; no filesystem writes or UML business rules.
//! A target-specific adapter prepares typed data; Tera formats only text.
//! All rendering occurs before a Plan can be published.
use serde::Serialize;
use tera::{Context, Tera};

/// Isolated template pack. Independent engines can provide different packs
/// without modifying Bellatrix's graph rules or GPUI host.
pub struct TemplatePack {
    tera: Tera,
}

impl TemplatePack {
    pub fn embedded<'a>(
        templates: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Result<Self, tera::Error> {
        let mut tera = Tera::default();
        // These templates emit source code and configuration, never HTML.
        // HTML exporters must configure escaping in their own template pack.
        tera.autoescape_on(Vec::<&str>::new());
        for (name, body) in templates {
            tera.add_raw_template(name, body)?;
        }
        Ok(Self { tera })
    }

    pub fn render<T: Serialize>(
        &self, template: &str, data: &T,
    ) -> Result<String, tera::Error> {
        let context = Context::from_serialize(data)?;
        self.tera.render(template, &context)
    }

    /// Replaces a registered target template, e.g. an SDK/plugin-specific
    /// C#, TypeScript, JavaScript or Rust style. Does not alter model rules.
    pub fn override_template(
        &mut self, name: &str, body: &str,
    ) -> Result<(), tera::Error> {
        self.tera.add_raw_template(name, body)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.tera.contains_template(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    #[derive(Serialize)]
    struct Decl<'a> { name: &'a str, fields: Vec<&'a str> }

    #[test]
    fn target_language_templates_receive_typed_data_without_html_escaping() {
        let engine = TemplatePack::embedded([
            ("ts/class.tera", "export class {{ name }} { {% for f in fields %}{{ f }}; {% endfor %}}"),
            ("cs/class.tera", "public class {{ name }} { {% for f in fields %}{{ f }}; {% endfor %}}"),
        ]).unwrap();
        let model = Decl { name: "Task", fields: vec!["title: string"] };
        assert!(engine.render("ts/class.tera", &model).unwrap().contains("title: string;"));
        assert!(engine.render("cs/class.tera", &model).unwrap().contains("public class Task"));
    }

    #[test]
    fn source_is_not_html_escaped_and_bad_templates_fail_before_publication() {
        let mut engine = TemplatePack::embedded([("txt", "{{ code }}")]).unwrap();
        assert_eq!(engine.render("txt", &serde_json::json!({"code": "Vec<&str>"})).unwrap(), "Vec<&str>");
        assert!(engine.override_template("broken", "{{ unknown").is_err());
    }
}
