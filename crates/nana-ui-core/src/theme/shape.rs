//! [`ShapeRecipe`]: which radius step a built-in component's corners take.
//!
//! A component used to name its own [`RadiusTier`] (a button said `Sm`), so
//! the theme could move how round `Sm` is but never which step a button
//! sits on: a design system whose buttons and tags are pills had to restyle
//! every one of them by hand, and lost that the moment a theme swap rebuilt
//! the view. The recipe moves the choice of step to the theme. A component
//! names its [`ShapeRole`] on the node (`NodeStyle::shape`); the installed
//! theme's recipe picks the step and its metrics the pixels, both at style
//! resolution, so a theme installed later reaches buttons already on screen.
//!
//! A step a node names itself (`NodeStyle::radius`, `corner_radii`) or a
//! number it writes over the component's style still wins: the recipe only
//! answers for corners nobody else decided.

use super::{RadiusTier, ThemeMetrics};

/// Which built-in component's corner the [`ShapeRecipe`] decides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShapeRole {
    /// `Button`, every kind and size.
    Button,
    /// `IconButton`, compact heading tools included.
    IconButton,
    /// The quiet label `tag` / `tag_frame` draws.
    Tag,
}

impl ShapeRole {
    pub const ALL: [Self; 3] = [Self::Button, Self::IconButton, Self::Tag];
}

/// The radius step each [`ShapeRole`] takes. The built-in value is what the
/// components named themselves before the recipe existed, so a theme that
/// does not set one looks exactly as before.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShapeRecipe {
    pub button: RadiusTier,
    pub icon_button: RadiusTier,
    pub tag: RadiusTier,
}

impl ShapeRecipe {
    pub const DEFAULT: Self = Self {
        button: RadiusTier::Sm,
        icon_button: RadiusTier::Sm,
        tag: RadiusTier::Sm,
    };

    /// Every role on `tier`: `ShapeRecipe::uniform(RadiusTier::Full)` makes
    /// buttons, icon buttons and tags pills.
    pub const fn uniform(tier: RadiusTier) -> Self {
        Self {
            button: tier,
            icon_button: tier,
            tag: tier,
        }
    }

    /// The step `role` takes.
    pub const fn tier(&self, role: ShapeRole) -> RadiusTier {
        match role {
            ShapeRole::Button => self.button,
            ShapeRole::IconButton => self.icon_button,
            ShapeRole::Tag => self.tag,
        }
    }

    /// `role`'s corner in logical pixels under `metrics`.
    pub const fn resolve(&self, role: ShapeRole, metrics: ThemeMetrics) -> f32 {
        self.tier(role).resolve(metrics)
    }
}

impl Default for ShapeRecipe {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_shape_is_the_step_each_component_named_before() {
        for role in ShapeRole::ALL {
            assert_eq!(ShapeRecipe::DEFAULT.tier(role), RadiusTier::Sm);
        }
    }

    #[test]
    fn a_uniform_recipe_moves_every_role() {
        let pill = ShapeRecipe::uniform(RadiusTier::Full);
        for role in ShapeRole::ALL {
            assert_eq!(pill.tier(role), RadiusTier::Full);
            assert_eq!(
                pill.resolve(role, super::super::UI_METRICS),
                super::super::FULL_RADIUS
            );
        }
    }
}
