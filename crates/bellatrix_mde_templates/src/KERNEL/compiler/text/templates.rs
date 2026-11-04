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
