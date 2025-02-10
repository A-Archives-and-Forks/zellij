mod active_component;
mod pages;
use zellij_tile::prelude::*;

use pages::Page;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

const UI_ROWS: usize = 20;
const UI_COLUMNS: usize = 90;

#[derive(Debug)]
struct App {
    active_page: Page,
    link_executable: Rc<RefCell<String>>,
    zellij_version: Rc<RefCell<String>>,
    base_mode: Rc<RefCell<InputMode>>,
    tab_rows: usize,
    tab_columns: usize,
    own_plugin_id: Option<u32>,
    is_release_notes: bool,
}

impl Default for App {
    fn default() -> Self {
        let link_executable = Rc::new(RefCell::new("".to_owned()));
        let zellij_version = Rc::new(RefCell::new("".to_owned()));
        let base_mode = Rc::new(RefCell::new(Default::default()));
        App {
            active_page: Page::new_main_screen(
                link_executable.clone(),
                "".to_owned(),
                base_mode.clone(),
            ),
            link_executable,
            zellij_version,
            base_mode,
            tab_rows: 0,
            tab_columns: 0,
            own_plugin_id: None,
            is_release_notes: false,
        }
    }
}

register_plugin!(App);

impl ZellijPlugin for App {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.is_release_notes = configuration
            .get("is_release_notes")
            .map(|v| v == "true")
            .unwrap_or(false);
        subscribe(&[
            EventType::Key,
            EventType::Mouse,
            EventType::ModeUpdate,
            EventType::RunCommandResult,
            EventType::TabUpdate,
        ]);
        let own_plugin_id = get_plugin_ids().plugin_id;
        self.own_plugin_id = Some(own_plugin_id);
        *self.zellij_version.borrow_mut() = get_zellij_version();
        self.change_own_title();
        self.query_link_executable();
        self.active_page = Page::new_main_screen(
            self.link_executable.clone(),
            self.zellij_version.borrow().clone(),
            self.base_mode.clone(),
        );
    }
    fn update(&mut self, event: Event) -> bool {
        let mut should_render = false;
        match event {
            Event::TabUpdate(tab_info) => {
                self.center_own_pane(tab_info);
            },
            Event::Mouse(mouse_event) => {
                should_render = self.handle_mouse_event(mouse_event);
            },
            Event::ModeUpdate(mode_info) => {
                if let Some(base_mode) = mode_info.base_mode {
                    should_render = self.update_base_mode(base_mode);
                }
            },
            Event::RunCommandResult(exit_code, _stdout, _stderr, context) => {
                let is_xdg_open = context.get("xdg_open_cli").is_some();
                let is_open = context.get("open_cli").is_some();
                if is_xdg_open {
                    if exit_code == Some(0) {
                        self.update_link_executable("xdg-open".to_owned());
                    }
                } else if is_open {
                    if exit_code == Some(0) {
                        self.update_link_executable("open".to_owned());
                    }
                }
            },
            Event::Key(key) => {
                should_render = self.handle_key(key);
            },
            _ => {},
        }
        should_render
    }
    fn render(&mut self, rows: usize, cols: usize) {
        self.active_page.render(rows, cols);
    }
}

impl App {
    pub fn change_own_title(&mut self) {
        if let Some(own_plugin_id) = self.own_plugin_id {
            if self.is_release_notes {
                rename_plugin_pane(
                    own_plugin_id,
                    format!("Release Notes {}", self.zellij_version.borrow()),
                );
            } else {
                rename_plugin_pane(own_plugin_id, "About Zellij");
            }
        }
    }
    pub fn query_link_executable(&self) {
        let mut xdg_open_context = BTreeMap::new();
        xdg_open_context.insert("xdg_open_cli".to_owned(), String::new());
        run_command(&["xdg-open", "--help"], xdg_open_context);
        let mut open_context = BTreeMap::new();
        open_context.insert("open_cli".to_owned(), String::new());
        run_command(&["open", "--help"], open_context);
    }
    pub fn update_link_executable(&mut self, new_link_executable: String) {
        *self.link_executable.borrow_mut() = new_link_executable;
    }
    pub fn update_base_mode(&mut self, new_base_mode: InputMode) -> bool {
        let mut should_render = false;
        if *self.base_mode.borrow() != new_base_mode {
            should_render = true;
        }
        *self.base_mode.borrow_mut() = new_base_mode;
        should_render
    }
    pub fn handle_mouse_event(&mut self, mouse_event: Mouse) -> bool {
        let mut should_render = false;
        match mouse_event {
            Mouse::LeftClick(line, column) => {
                if let Some(new_page) = self
                    .active_page
                    .handle_mouse_left_click(column, line as usize)
                {
                    self.active_page = new_page;
                    should_render = true;
                }
            },
            Mouse::Hover(line, column) => {
                should_render = self.active_page.handle_mouse_hover(column, line as usize);
            },
            _ => {},
        }
        should_render
    }
    pub fn handle_key(&mut self, key: KeyWithModifier) -> bool {
        let mut should_render = false;
        if key.bare_key == BareKey::Enter && key.has_no_modifiers() {
            if let Some(new_page) = self.active_page.handle_selection() {
                self.active_page = new_page;
                should_render = true;
            }
        } else if key.bare_key == BareKey::Esc && key.has_no_modifiers() {
            if self.active_page.is_main_screen {
                close_self();
            } else {
                self.active_page = Page::new_main_screen(
                    self.link_executable.clone(),
                    self.zellij_version.borrow().clone(),
                    self.base_mode.clone(),
                );
                should_render = true;
            }
        } else {
            should_render = self.active_page.handle_key(key);
        }
        should_render
    }
    fn center_own_pane(&mut self, tab_info: Vec<TabInfo>) {
        // we only take the size of the first tab because at the time of writing this is
        // identical to all tabs, but this might not always be the case...
        if let Some(first_tab) = tab_info.get(0) {
            let prev_tab_columns = self.tab_columns;
            let prev_tab_rows = self.tab_rows;
            self.tab_columns = first_tab.display_area_columns;
            self.tab_rows = first_tab.display_area_rows;
            if self.tab_columns != prev_tab_columns || self.tab_rows != prev_tab_rows {
                let desired_x_coords = self.tab_columns.saturating_sub(UI_COLUMNS) / 2;
                let desired_y_coords = self.tab_rows.saturating_sub(UI_ROWS) / 2;
                change_floating_panes_coordinates(vec![(
                    PaneId::Plugin(self.own_plugin_id.unwrap()),
                    FloatingPaneCoordinates::new(
                        Some(desired_x_coords.to_string()),
                        Some(desired_y_coords.to_string()),
                        Some(UI_COLUMNS.to_string()),
                        Some(UI_ROWS.to_string()),
                        None,
                    )
                    .unwrap(),
                )]);
            }
        }
    }
}
