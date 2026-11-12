//! Offline UI translations. Replay values and SQL identifiers are never translated.
use std::{cell::RefCell, collections::BTreeMap, sync::OnceLock};
pub const LANGUAGES: &[(&str, &str)] = &[
    ("de", "Deutsch"),
    ("en", "English"),
    ("es", "Español"),
    ("pl", "Polski"),
    ("fr", "Français"),
    ("cs", "Čeština"),
    ("uk", "Українська"),
    ("ru", "Русский"),
    ("it", "Italiano"),
    ("tr", "Türkçe"),
    ("pt", "Português"),
    ("hu", "Magyar"),
    ("ro", "Română"),
];
thread_local! {static LANGUAGE:RefCell<String>=RefCell::new("de".into());}
type Catalog = BTreeMap<String, BTreeMap<String, String>>;
fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("locales/catalog.json"))
            .expect("validated translation catalog")
    })
}
pub fn valid_language(code: &str) -> bool {
    LANGUAGES.iter().any(|(c, _)| *c == code)
}
pub fn set_language(code: &str) {
    LANGUAGE.with(|v| {
        if v.borrow().as_str() != code {
            *v.borrow_mut() = if valid_language(code) { code } else { "de" }.into();
        }
    });
}
pub fn static_text(source: &'static str) -> &'static str {
    LANGUAGE.with(|v| {
        let lang = v.borrow();
        if lang.as_str() == "de" {
            return source;
        }
        catalog()
            .get(source)
            .and_then(|m| m.get(lang.as_str()).or_else(|| m.get("en")))
            .map(String::as_str)
            .unwrap_or(source)
    })
}
pub fn text(source: impl AsRef<str>) -> String {
    let source = source.as_ref();
    LANGUAGE.with(|v| {
        let lang = v.borrow();
        if lang.as_str() == "de" {
            return source.to_owned();
        }
        if let Some(translated) = catalog()
            .get(source)
            .and_then(|m| m.get(lang.as_str()).or_else(|| m.get("en")))
        {
            return translated.clone();
        }
        if source.contains(' ') {
            for (template, translations) in catalog() {
                if template.contains('{') {
                    if let Some(target) = translations
                        .get(lang.as_str())
                        .or_else(|| translations.get("en"))
                    {
                        if let Some(result) = interpolate(template, target, source) {
                            return result;
                        }
                    }
                }
            }
        }
        source.to_owned()
    })
}
/// Translate a Rust format template while preserving its already formatted values.
pub fn formatted(template: &str, formatted: impl Into<eframe::egui::WidgetText>) -> String {
    let formatted = formatted.into().text().to_owned();
    LANGUAGE.with(|v| {
        let lang = v.borrow();
        if lang.as_str() == "de" {
            return formatted;
        }
        let Some(target) = catalog()
            .get(template)
            .and_then(|m| m.get(lang.as_str()).or_else(|| m.get("en")))
        else {
            return formatted;
        };
        interpolate(template, target, &formatted).unwrap_or(formatted)
    })
}
fn parts(template: &str) -> Vec<(bool, String)> {
    let mut parts = Vec::new();
    let mut literal = String::new();
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' && chars.peek() == Some(&'{') {
            chars.next();
            literal.push('{');
        } else if c == '}' && chars.peek() == Some(&'}') {
            chars.next();
            literal.push('}');
        } else if c == '{' {
            parts.push((false, std::mem::take(&mut literal)));
            let mut key = String::new();
            for c in chars.by_ref() {
                if c == '}' {
                    break;
                }
                key.push(c);
            }
            parts.push((true, key));
        } else {
            literal.push(c);
        }
    }
    parts.push((false, literal));
    parts
}
fn interpolate(source: &str, target: &str, value: &str) -> Option<String> {
    let segments = parts(source);
    let mut rest = value;
    let mut values = BTreeMap::new();
    let mut position = 0;
    for (i, (placeholder, part)) in segments.iter().enumerate() {
        if !placeholder {
            rest = rest.strip_prefix(part)?;
        } else {
            let next = segments.get(i + 1).map(|(_, p)| p.as_str()).unwrap_or("");
            let length = if next.is_empty() {
                if i + 2 < segments.len() {
                    return None;
                } else {
                    rest.len()
                }
            } else {
                rest.find(next)?
            };
            let key = if part.is_empty() {
                let key = format!("#{position}");
                position += 1;
                key
            } else {
                part.clone()
            };
            values.insert(key, rest[..length].to_owned());
            rest = &rest[length..];
        }
    }
    if !rest.is_empty() {
        return None;
    }
    let mut output = String::new();
    position = 0;
    for (placeholder, part) in parts(target) {
        if placeholder {
            let key = if part.is_empty() {
                let key = format!("#{position}");
                position += 1;
                key
            } else {
                part
            };
            output.push_str(values.get(&key)?);
        } else {
            output.push_str(&part);
        }
    }
    Some(output)
}
pub fn widget(value: impl Into<eframe::egui::WidgetText>) -> eframe::egui::WidgetText {
    match value.into() {
        eframe::egui::WidgetText::Text(s) => text(s).into(),
        other => other,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn language_selection_fallback_and_values() {
        set_language("en");
        assert_eq!(text("Schaden"), "Damage");
        assert_eq!(text("Friend_of_Misery"), "Friend_of_Misery");
        assert_eq!(
            interpolate("Seite {} / {}", "Page {} / {}", "Seite 2 / 10").unwrap(),
            "Page 2 / 10"
        );
        assert_eq!(
            interpolate("{n} Gefechte", "{n} battles", "42 Gefechte").unwrap(),
            "42 battles"
        );
        set_language("de");
    }
    #[test]
    fn all_catalog_placeholders_and_languages_are_valid() {
        for (source, translations) in catalog() {
            let mut expected: Vec<_> = parts(source)
                .into_iter()
                .filter(|(p, _)| *p)
                .map(|(_, s)| s)
                .collect();
            expected.sort();
            for (code, target) in translations {
                assert!(valid_language(code));
                let mut actual: Vec<_> = parts(target)
                    .into_iter()
                    .filter(|(p, _)| *p)
                    .map(|(_, s)| s)
                    .collect();
                actual.sort();
                assert_eq!(expected, actual, "{code}: {source}");
            }
        }
        assert_eq!(LANGUAGES.len(), 13);
    }
}
pub fn rich(value: impl Into<eframe::egui::RichText>) -> eframe::egui::RichText {
    let value = value.into();
    let translated = text(value.text());
    if translated == value.text() {
        value
    } else {
        eframe::egui::RichText::new(translated)
    }
}
#[macro_export]
macro_rules! tr_format {
    ($template:literal $($args:tt)*) => { $crate::i18n::formatted($template,format!($template $($args)*)) };
}
