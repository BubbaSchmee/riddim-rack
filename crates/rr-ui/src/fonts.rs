//! Embedded fonts (both SIL OFL, licences in `assets/fonts/`): Barlow for labels, titles and
//! the nameplate; JetBrains Mono for numeric readouts.

use egui::{Context, FontData, FontDefinitions, FontFamily};

const BARLOW_MEDIUM: &[u8] = include_bytes!("../assets/fonts/Barlow-Medium.ttf");
const BARLOW_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Barlow-SemiBold.ttf");
const MONO_REGULAR: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");
const MONO_MEDIUM: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Medium.ttf");

/// Families: Proportional = Barlow Medium, Monospace = JetBrains Mono Regular,
/// "barlow-semibold", "mono-medium".
pub fn definitions() -> FontDefinitions {
    let mut fd = FontDefinitions::empty();
    for (name, bytes) in [
        ("barlow-medium", BARLOW_MEDIUM),
        ("barlow-semibold", BARLOW_SEMIBOLD),
        ("mono-regular", MONO_REGULAR),
        ("mono-medium", MONO_MEDIUM),
    ] {
        fd.font_data.insert(name.to_owned(), std::sync::Arc::new(FontData::from_static(bytes)));
    }
    let fam = |names: &[&str]| names.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    fd.families.insert(FontFamily::Proportional, fam(&["barlow-medium", "mono-regular"]));
    fd.families.insert(FontFamily::Monospace, fam(&["mono-regular", "barlow-medium"]));
    fd.families.insert(FontFamily::Name("barlow-semibold".into()), fam(&["barlow-semibold", "barlow-medium"]));
    fd.families.insert(FontFamily::Name("mono-medium".into()), fam(&["mono-medium", "mono-regular"]));
    fd
}

pub fn install(ctx: &Context) {
    ctx.set_fonts(definitions());
}

#[cfg(test)]
mod tests {
    #[test]
    fn families_resolve_to_registered_fonts() {
        let fd = super::definitions();
        for names in fd.families.values() {
            assert!(names.iter().all(|n| fd.font_data.contains_key(n)));
        }
        assert_eq!(fd.families.len(), 4);
    }
}
