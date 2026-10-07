use std::collections::{BTreeSet, HashMap};

/// Represents an atomic, undoable page organisation operation.
#[derive(Debug, Clone, PartialEq)]
pub enum OrganizeCommand {
    /// Permutation of page indices changed from `old_order` to `new_order`.
    Reorder {
        old_order: Vec<usize>,
        new_order: Vec<usize>,
    },
    /// Rotations changed for a set of pages (page_index -> (old_rotation, new_rotation)).
    Rotate { deltas: HashMap<usize, (i32, i32)> },
    /// Deleted pages: stores (original_index, page_number) to allow restoration on undo.
    Delete { deleted: Vec<(usize, usize)> },
    /// Inserted a blank page at `inserted_index`.
    InsertBlank { inserted_index: usize },
}

/// State controller for the Interactive "Organize Pages" Card Grid (Light-Table Mode).
#[derive(Debug, Clone)]
pub struct PageOrganizerState {
    /// Number of pages in the active document.
    pub total_pages: usize,
    /// Current logical ordering of pages (index -> original document page).
    pub page_order: Vec<usize>,
    /// Accumulated page rotations (page_index -> angle in degrees: 0, 90, 180, 270).
    pub page_rotations: HashMap<usize, i32>,
    /// Currently selected pages in the light-table card grid.
    pub selected_pages: BTreeSet<usize>,
    /// Active drag source card index during a drag-and-drop reorder gesture.
    pub dragging_index: Option<usize>,
    /// Drop insertion target indicator index.
    pub drop_target_index: Option<usize>,
    /// Last clicked page index (anchor for Shift-click multi-selection).
    pub selection_anchor: Option<usize>,
    /// Full Undo history stack for page organisation actions.
    pub undo_stack: Vec<OrganizeCommand>,
    /// Redo history stack.
    pub redo_stack: Vec<OrganizeCommand>,
    /// Zoom level / scale of the card grid thumbnails (100% = standard card size).
    pub grid_scale: f32,
}

impl PageOrganizerState {
    /// Creates a new PageOrganizerState for a document with `total_pages`.
    pub fn new(total_pages: usize) -> Self {
        let page_order = (0..total_pages).collect();
        Self {
            total_pages,
            page_order,
            page_rotations: HashMap::new(),
            selected_pages: BTreeSet::new(),
            dragging_index: None,
            drop_target_index: None,
            selection_anchor: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            grid_scale: 1.0,
        }
    }

    /// Handles card selection with multi-select semantics (Single click, Ctrl/Cmd click, Shift click).
    pub fn handle_card_click(&mut self, page_idx: usize, is_ctrl: bool, is_shift: bool) {
        if page_idx >= self.page_order.len() {
            return;
        }

        if is_shift {
            if let Some(anchor) = self.selection_anchor {
                let start = anchor.min(page_idx);
                let end = anchor.max(page_idx);
                if !is_ctrl {
                    self.selected_pages.clear();
                }
                for i in start..=end {
                    self.selected_pages.insert(i);
                }
            } else {
                self.selected_pages.insert(page_idx);
                self.selection_anchor = Some(page_idx);
            }
        } else if is_ctrl {
            if self.selected_pages.contains(&page_idx) {
                self.selected_pages.remove(&page_idx);
            } else {
                self.selected_pages.insert(page_idx);
            }
            self.selection_anchor = Some(page_idx);
        } else {
            // Normal click: select only this card
            self.selected_pages.clear();
            self.selected_pages.insert(page_idx);
            self.selection_anchor = Some(page_idx);
        }
    }

    /// Selects all pages in the grid (`Ctrl + A`).
    pub fn select_all(&mut self) {
        self.selected_pages.clear();
        for i in 0..self.page_order.len() {
            self.selected_pages.insert(i);
        }
    }

    /// Clears active selection.
    pub fn clear_selection(&mut self) {
        self.selected_pages.clear();
        self.selection_anchor = None;
    }

    /// Rotates currently selected pages by `delta_degrees` (e.g. 90, -90, 180).
    pub fn rotate_selected(&mut self, delta_degrees: i32) {
        if self.selected_pages.is_empty() {
            return;
        }

        let mut deltas = HashMap::new();
        for &page_idx in &self.selected_pages {
            let current = *self.page_rotations.get(&page_idx).unwrap_or(&0);
            let updated = (current + delta_degrees).rem_euclid(360);
            deltas.insert(page_idx, (current, updated));
            self.page_rotations.insert(page_idx, updated);
        }

        self.undo_stack.push(OrganizeCommand::Rotate { deltas });
        self.redo_stack.clear();
    }

    /// Deletes currently selected pages.
    pub fn delete_selected(&mut self) {
        if self.selected_pages.is_empty() || self.page_order.is_empty() {
            return;
        }

        let mut deleted = Vec::new();
        let mut new_order = Vec::with_capacity(self.page_order.len());

        for (idx, &page) in self.page_order.iter().enumerate() {
            if self.selected_pages.contains(&idx) {
                deleted.push((idx, page));
            } else {
                new_order.push(page);
            }
        }

        self.page_order = new_order;
        self.total_pages = self.page_order.len();
        self.selected_pages.clear();
        self.selection_anchor = None;

        self.undo_stack.push(OrganizeCommand::Delete { deleted });
        self.redo_stack.clear();
    }

    /// Moves a card from `from_index` to `to_index` (drag & drop reorder).
    pub fn reorder_card(&mut self, from_index: usize, to_index: usize) {
        if from_index == to_index
            || from_index >= self.page_order.len()
            || to_index >= self.page_order.len()
        {
            return;
        }

        let old_order = self.page_order.clone();
        let moved = self.page_order.remove(from_index);
        self.page_order.insert(to_index, moved);
        let new_order = self.page_order.clone();

        // Update selection if the moved card was selected
        if self.selected_pages.remove(&from_index) {
            self.selected_pages.insert(to_index);
        }

        self.undo_stack.push(OrganizeCommand::Reorder {
            old_order,
            new_order,
        });
        self.redo_stack.clear();
    }

    /// Inserts a blank page at `target_index`.
    pub fn insert_blank_page(&mut self, target_index: usize) {
        let insert_at = target_index.min(self.page_order.len());
        // Synthetic blank page identifier (represented with usize::MAX sentinel)
        self.page_order.insert(insert_at, usize::MAX);
        self.total_pages = self.page_order.len();

        self.undo_stack.push(OrganizeCommand::InsertBlank {
            inserted_index: insert_at,
        });
        self.redo_stack.clear();
    }

    /// Reverts the most recent page organisation action (`Ctrl + Z`).
    pub fn undo(&mut self) -> bool {
        let Some(cmd) = self.undo_stack.pop() else {
            return false;
        };

        match &cmd {
            OrganizeCommand::Reorder { old_order, .. } => {
                self.page_order = old_order.clone();
            }
            OrganizeCommand::Rotate { deltas } => {
                for (&idx, &(old_rot, _)) in deltas {
                    self.page_rotations.insert(idx, old_rot);
                }
            }
            OrganizeCommand::Delete { deleted } => {
                let mut restored = self.page_order.clone();
                for &(idx, page) in deleted {
                    if idx <= restored.len() {
                        restored.insert(idx, page);
                    } else {
                        restored.push(page);
                    }
                }
                self.page_order = restored;
                self.total_pages = self.page_order.len();
            }
            OrganizeCommand::InsertBlank { inserted_index } => {
                if *inserted_index < self.page_order.len() {
                    self.page_order.remove(*inserted_index);
                    self.total_pages = self.page_order.len();
                }
            }
        }

        self.redo_stack.push(cmd);
        true
    }

    /// Re-applies the most recently reverted action (`Ctrl + Y`).
    pub fn redo(&mut self) -> bool {
        let Some(cmd) = self.redo_stack.pop() else {
            return false;
        };

        match &cmd {
            OrganizeCommand::Reorder { new_order, .. } => {
                self.page_order = new_order.clone();
            }
            OrganizeCommand::Rotate { deltas } => {
                for (&idx, &(_, new_rot)) in deltas {
                    self.page_rotations.insert(idx, new_rot);
                }
            }
            OrganizeCommand::Delete { deleted } => {
                let to_remove: BTreeSet<usize> = deleted.iter().map(|&(idx, _)| idx).collect();
                self.page_order = self
                    .page_order
                    .iter()
                    .enumerate()
                    .filter(|(idx, _)| !to_remove.contains(idx))
                    .map(|(_, &p)| p)
                    .collect();
                self.total_pages = self.page_order.len();
            }
            OrganizeCommand::InsertBlank { inserted_index } => {
                if *inserted_index <= self.page_order.len() {
                    self.page_order.insert(*inserted_index, usize::MAX);
                    self.total_pages = self.page_order.len();
                }
            }
        }

        self.undo_stack.push(cmd);
        true
    }
}
