/// Theme palette choices tailored for long reading sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ReadModeTheme {
    #[default]
    Default,
    /// Warm, soothing parchment tone.
    Sepia,
    /// High-contrast dark theme designed to reduce eye fatigue.
    DarkNight,
    /// Low-contrast green-tinted paper tone.
    EyeCareGreen,
    /// Monochromatic grayscale mode.
    PurePaper,
}

/// Layout alignment for facing pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum TwoUpLayout {
    #[default]
    /// Standard side-by-side spread: [Page 1 | Page 2], [Page 3 | Page 4]
    Continuous,
    /// Book mode with Cover Offset: [Page 1 (Cover)] alone, then [Page 2 | Page 3], [Page 4 | Page 5]
    BookCoverOffset,
}

/// Configuration and state for Distraction-Free "Read Mode".
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReadModeConfig {
    /// Whether Distraction-Free mode is currently active (hides ribbons, sidebars, tabs, status bars).
    pub is_active: bool,
    /// Whether Two-Up facing pages layout is enabled.
    pub two_up_enabled: bool,
    /// Two-Up spread style (Standard or Book cover offset).
    pub two_up_layout: TwoUpLayout,
    /// Reading color theme.
    pub theme: ReadModeTheme,
    /// Zoom level specifically maintained for Read Mode (e.g. 1.0 = Fit Page).
    pub read_zoom: f32,
    /// Auto-hide cursor after inactivity (milliseconds).
    pub auto_hide_cursor_ms: u64,
}

impl Default for ReadModeConfig {
    fn default() -> Self {
        Self {
            is_active: false,
            two_up_enabled: false,
            two_up_layout: TwoUpLayout::BookCoverOffset,
            theme: ReadModeTheme::Default,
            read_zoom: 1.0,
            auto_hide_cursor_ms: 2500,
        }
    }
}

impl ReadModeConfig {
    /// Toggles Distraction-Free reading mode on/off (`F10` or ribbon button).
    pub fn toggle_active(&mut self) -> bool {
        self.is_active = !self.is_active;
        self.is_active
    }

    /// Toggles Two-Up facing pages spread mode.
    pub fn toggle_two_up(&mut self) -> bool {
        self.two_up_enabled = !self.two_up_enabled;
        self.two_up_enabled
    }

    /// Sets the reading theme.
    pub fn set_theme(&mut self, theme: ReadModeTheme) {
        self.theme = theme;
    }

    /// Computes the left and right page indices for a given current page index and total page count.
    ///
    /// Respects the book cover offset rule where Page 0 is presented singly as the book cover.
    pub fn compute_spread_pages(
        &self,
        current_page: usize,
        total_pages: usize,
    ) -> (usize, Option<usize>) {
        if total_pages == 0 {
            return (0, None);
        }

        if !self.two_up_enabled {
            return (current_page.min(total_pages - 1), None);
        }

        match self.two_up_layout {
            TwoUpLayout::BookCoverOffset => {
                if current_page == 0 {
                    // Page 0 (cover) rendered standalone centered
                    (0, None)
                } else {
                    // Facing pages: odd pages on left (1, 3, 5...), even on right (2, 4, 6...)
                    let left = if current_page % 2 == 1 {
                        current_page
                    } else {
                        current_page.saturating_sub(1)
                    };
                    let right = if left + 1 < total_pages {
                        Some(left + 1)
                    } else {
                        None
                    };
                    (left, right)
                }
            }
            TwoUpLayout::Continuous => {
                // Facing pairs: (0, 1), (2, 3), (4, 5)...
                let left = if current_page % 2 == 0 {
                    current_page
                } else {
                    current_page.saturating_sub(1)
                };
                let right = if left + 1 < total_pages {
                    Some(left + 1)
                } else {
                    None
                };
                (left, right)
            }
        }
    }
}
