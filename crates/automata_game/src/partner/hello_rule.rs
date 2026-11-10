use crate::character::Character;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeroIdentity {
    pub handle: u32,
    /// The character of the hero's model, when the model is one of the three.
    pub character: Option<Character>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HelloRule {
    was_connected: bool,
    had_hero: bool,
    hero_handle_told_for: Option<u32>,
    character_told: Option<Character>,
    answered_the_partner: bool,
}

impl HelloRule {
    /// The character to tell now, if any: on connect, when a hero appears or changes, when its
    /// character changes, and once more when the other game is first heard.
    pub fn character_to_tell(
        &mut self,
        connected: bool,
        link_setting: Character,
        hero: Option<HeroIdentity>,
        partner_heard: bool,
    ) -> Option<Character> {
        if !connected {
            *self = Self::default();
            return None;
        }
        let just_connected = !self.was_connected;
        self.was_connected = true;
        let hero_appeared = hero.is_some() && !self.had_hero;
        self.had_hero = hero.is_some();
        let hero_handle = hero.map(|hero| hero.handle);
        let hero_changed = hero_handle.is_some()
            && self.hero_handle_told_for.is_some()
            && hero_handle != self.hero_handle_told_for;
        if hero_handle.is_some() {
            self.hero_handle_told_for = hero_handle;
        }
        let character = hero.and_then(|hero| hero.character).unwrap_or(link_setting);
        let character_changed = self.character_told.is_some_and(|told| told != character);
        // Our first hello is lost when the other game was not linked to its launcher yet.
        let answers_the_partner = partner_heard && !self.answered_the_partner;
        self.answered_the_partner |= partner_heard;
        let tell = just_connected
            || hero_appeared
            || hero_changed
            || character_changed
            || answers_the_partner;
        if tell {
            self.character_told = Some(character);
        }
        tell.then_some(character)
    }
}
