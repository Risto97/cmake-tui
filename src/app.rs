use std::{collections::HashMap, path::PathBuf};

use color_eyre::Result;
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Layout, Rect, Flex},
    style::{
        Color, Modifier, Style, Stylize,
        palette::tailwind::{BLUE, SLATE},
    },
    symbols,
    text::Line,
    widgets::{
        Block, Borders, HighlightSpacing, Padding, Paragraph, StatefulWidget, 
        Widget, Wrap, Table, Row, Cell, TableState, Clear
    },
};

use crate::cache_parser::{CacheVar, VarType, parse_cmake_cache};

const TODO_HEADER_STYLE: Style = Style::new().fg(SLATE.c100).bg(BLUE.c800);
const NORMAL_ROW_BG: Color = SLATE.c950;
const ALT_ROW_BG_COLOR: Color = SLATE.c900;
const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);
const TEXT_FG_COLOR: Color = SLATE.c200;
// const COMPLETED_TEXT_FG_COLOR: Color = GREEN.c500;

#[derive(PartialEq)]
enum AppMode {
    Scroll,
    ValueEdit,
    SearchInput,
}

pub struct App {
    should_exit: bool,
    var_list: CacheVarList,
    mode: AppMode,
    show_advanced: bool,

    search_input: String,
    cursor_pos: usize,
}

struct CacheVarTui {
    var: CacheVar,
    new_val: String,
}

impl From<CacheVar> for CacheVarTui {
    fn from(var: CacheVar) -> Self {
        CacheVarTui {
            new_val: var.value.clone(),
            var: var,
        }
    }
}

struct CacheVarList {
    vars: Vec<CacheVarTui>,
    row_idx_var_idx_map: HashMap<usize, usize>,
    longest_name: usize,
    state: TableState,
}

impl App {
    pub fn new(build_dir: PathBuf) -> Self {
        let vec: Vec<CacheVar> =
            parse_cmake_cache(build_dir).unwrap_or_default();

        let tui_vec: Vec<CacheVarTui> = vec
                    .into_iter()
                    .map(CacheVarTui::from) // Uses the impl From we just wrote
                    .collect();

        let max_len = tui_vec
            .iter()
            .map(|i| i.var.name.chars().count())
            .max()
            .unwrap_or(100); // Default fallback width
        
        let var_list = CacheVarList {
            vars: tui_vec,
            row_idx_var_idx_map: HashMap::new(),
            longest_name: max_len,
            state: TableState::default(),
        };

        Self {
            should_exit: false,
            var_list: var_list,
            mode: AppMode::Scroll,
            show_advanced: false,

            search_input: "".to_string(),
            cursor_pos: 0,
        }
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.rebuild_idx_map();
        while !self.should_exit {
            terminal.draw(|frame| frame.render_widget(&mut self, frame.area()))?;
            if let Event::Key(key) = event::read()? {
                self.handle_key(key);
            };
        }
        Ok(())
    }

    fn handle_scroll_mode_key(&mut self, key: KeyEvent){
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc  => self.should_exit = true,
            // KeyCode::Char('h') | KeyCode::Left => self.select_none(),
            KeyCode::Char('j') | KeyCode::Down => self.select_next(),
            KeyCode::Char('k') | KeyCode::Up   => self.select_previous(),
            KeyCode::Char('g') | KeyCode::Home => self.select_first(),
            KeyCode::Char('G') | KeyCode::End  => self.select_last(),
            KeyCode::Char('t')  => self.toggle_show_advanced(),
            KeyCode::Enter => self.edit_value(),
            KeyCode::Char(' ') => self.cycle_value(),
            KeyCode::Char('/') => self.search_var(),
            KeyCode::Char('n') => self.select_next_search_result(),
            _ => {}
        }
    }

    fn rebuild_idx_map(&mut self){
        self.var_list.row_idx_var_idx_map.clear();
        for (original_idx, var) in self.var_list.vars.iter().enumerate(){
            if self.show_advanced || !var.var.advanced {
                let row_idx = self.var_list.row_idx_var_idx_map.len();
                self.var_list.row_idx_var_idx_map.insert(row_idx, original_idx);
            }
        }
    }

    // fn get_selected_var_idx(&self) -> Option<usize> {
    //     self.var_list.state.selected()
    //         .and_then(|row_idx| self.var_list.row_idx_var_idx_map.get(&row_idx))
    //         .copied()
    // }

    fn check_if_var_is_modified(&self, var: &CacheVarTui) -> bool {
        var.new_val != var.var.value
    }


    fn handle_search_input_mode_key(&mut self, key: KeyEvent){
        match key.code {
            KeyCode::Char(c) => {
                self.search_input.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
            }
            KeyCode::Esc  => {
                self.cursor_pos = 0;
                self.search_input.clear();
                self.mode = AppMode::Scroll;
            }
            // KeyCode::Char('h') | KeyCode::Left => self.select_none(),
            KeyCode::Backspace => {
                if self.search_input.len() > 0 {
                    self.search_input.pop();
                    self.cursor_pos -= 1;
                }
            }
            KeyCode::Left => {
                if self.cursor_pos > 0{
                    self.cursor_pos -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_pos < self.search_input.len() {
                    self.cursor_pos += 1;
                }
            }
            KeyCode::Enter => {
                self.mode = AppMode::Scroll;
                self.select_next_search_result();
            }
            _ => {}
        }
    }


    fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        if self.mode == AppMode::Scroll{
            self.handle_scroll_mode_key(key);
        } else if self.mode == AppMode::SearchInput {
            self.handle_search_input_mode_key(key);
        }
    }

    fn select_next_search_result(&mut self){
        if self.mode != AppMode::Scroll { return; }
        if self.search_input.is_empty() { return; }

        let query = self.search_input.to_lowercase();

        let start_row = self.var_list.state.selected().unwrap_or(0);
        let last_row = self
            .var_list
            .row_idx_var_idx_map
            .len()-1;

        // Search the list starting from the current row until the end.
        // Once it wraps to the end search again from the begining of the list to the start row
        let search_order = (start_row + 1..last_row).chain(0..=start_row);

        for row in search_order {
            let var_idx = *self.var_list.row_idx_var_idx_map.get(&row).unwrap();
            let var = &self.var_list.vars.get(var_idx).unwrap();
            if var.var.name.to_lowercase().starts_with(&query){
                self.var_list.state.select(Some(row));
                return
            }
        }
    }

    fn toggle_show_advanced(&mut self) {
        self.show_advanced = !self.show_advanced;
        self.rebuild_idx_map();
    }

    fn select_next(&mut self) {
        if self.mode != AppMode::Scroll {return}
        self.var_list.state.select_next();
    }
    fn select_previous(&mut self) {
        if self.mode != AppMode::Scroll {return}
        self.var_list.state.select_previous();
    }

    fn select_first(&mut self) {
        if self.mode != AppMode::Scroll {return}
        self.var_list.state.select_first();
    }

    fn select_last(&mut self) {
        if self.mode != AppMode::Scroll {return}
        self.var_list.state.select_last();
    }

    fn search_var(&mut self) {
        if self.mode != AppMode::Scroll {return}
        self.search_input.clear();
        self.cursor_pos = 0;
        self.mode = AppMode::SearchInput;
    }

    fn cycle_value(&mut self) {
        if self.mode != AppMode::Scroll {return}

        let var: &mut CacheVarTui = self.get_selected_var_mut().unwrap(); 

        if var.var.typ == VarType::Bool {
            var.new_val = CacheVar::toggle_bool(&var.new_val);
        } else if var.var.typ == VarType::Enum {
            var.new_val = var.var.cycle_enum(&var.new_val);
        }

    }

    fn edit_value(&mut self) {
        if self.mode == AppMode::ValueEdit {
            self.mode = AppMode::Scroll;

        } else if self.mode == AppMode::Scroll {
            if self.get_selected_var().unwrap().var.typ == VarType::Bool {
                // self.mode = AppMode::ValueEdit
            }
        }
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [title_area, main_area, help_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        let [list_area, footer_area] =
            Layout::vertical([Constraint::Fill(9), Constraint::Fill(1)]).areas(main_area);

        App::render_title_header(title_area, buf);
        App::render_help_footer(help_area, buf);
        self.render_var_table(list_area, buf);

        if self.mode != AppMode::SearchInput{
            self.render_selected_var(footer_area, buf);
        } else {
            self.render_search_footer(footer_area, buf);

        }

        self.render_popup(area, buf);
    }
}

impl App {
    fn render_title_header(area: Rect, buf: &mut Buffer) {
        Paragraph::new("CMake-TUI")
            .bold()
            .centered()
            .render(area, buf);
    }

    fn render_help_footer(area: Rect, buf: &mut Buffer) {
        Paragraph::new("Use ↓↑ to move, <Space> to cycle value, <Enter> to edit value, / to search, n to cycle search results, t to toggle advanced, g/G to go top/bottom.")
            .centered()
            .render(area, buf);
    }

    fn get_selected_var_mut(&mut self) -> Option<&mut CacheVarTui> {
        let row_idx = self.var_list.state.selected()?;
        let var_idx = *self.var_list.row_idx_var_idx_map.get(&row_idx)?;
        self.var_list.vars.get_mut(var_idx)
    }

    fn get_selected_var(&self) -> Option<&CacheVarTui> {
        let row_idx = self.var_list.state.selected()?;
        let var_idx = *self.var_list.row_idx_var_idx_map.get(&row_idx)?;
        self.var_list.vars.get(var_idx)
    }

    fn render_popup(&self, area: Rect, buf: &mut Buffer) {
        if self.mode != AppMode::ValueEdit {return};

        let var = self.get_selected_var().unwrap(); // TODO fix unwrap

        // Format the detailed content. Use Line::from(Vec<Span>) for rich text.
        let content = vec![
            Line::from(format!("Name: {}", var.var.name)).bold(),
            Line::from(format!("Type: {}", var.var.typ)),
            // Line::from(format!("Value: {}", var.value)),
            // Line::from(vec![
            //     "Description: ".bold(),
            //     // Assuming 'desc' field exists on CacheVar based on your prior commented code
            //     var.desc.clone().into(), 
            // ]),
        ];
        // let content = vec![Line::from(format!("Name")).bold()];

        // 2. Define the size and position of the popup
        let popup_area = popup_area(area, 20, 10); // 70% width, 50% height
        Clear.render(popup_area, buf);

        // // 3. Define the Block
        let block = Block::new()
            .title(Line::raw("Full Cache Variable Details").centered().bold())
            .borders(Borders::ALL)
            .border_style(Style::new().fg(BLUE.c500))
            .bg(NORMAL_ROW_BG); // Dark background

        // 4. Render the Content Paragraph
        Paragraph::new(content)
            .block(block)
            .fg(TEXT_FG_COLOR)
            .wrap(Wrap { trim: false })
            .render(popup_area, buf);
    }

    // --- NEW TABLE RENDERING LOGIC ---
    fn render_var_table(&mut self, area: Rect, buf: &mut Buffer) {
        // 1. Define the Container Block
        let block = Block::new()
            .title(Line::raw(" Cache Entries ").left_aligned())
            .borders(Borders::TOP)
            .border_set(symbols::border::EMPTY)
            .border_style(TODO_HEADER_STYLE)
            .bg(NORMAL_ROW_BG);

        // 2. Define the Header Row
        let header = Row::new(vec![
            Cell::from("Name"),
            Cell::from("Type"),
            Cell::from("Value")
        ])
        .style(TODO_HEADER_STYLE)
        .height(1)
        .bottom_margin(1); 


        // 3. Define the Rows from tui_vars
        let rows: Vec<Row> = self
            .var_list
            .vars
            .iter()
            .filter(|var| self.show_advanced || !var.var.advanced)
            .enumerate()
            .map(|(i, var)| {
                let color = alternate_colors(i);

                let name_label = if self.check_if_var_is_modified(var) {
                    format!("*{}", var.var.name)
                } else {
                    format!(" {}", var.var.name)
                };
                
                // Assuming var.var.name, var.var.typ, var.var.value implement Display
                Row::new(vec![
                    Cell::from(name_label),
                    Cell::from(var.var.typ.to_string()), 
                    Cell::from(var.new_val.to_string()),
                ])
                .style(Style::new().bg(color).fg(TEXT_FG_COLOR))
            })
            .collect();

        // 4. Define Column Widths
        // We use the calculated longest_name for the first column
        let widths = [
            Constraint::Length(self.var_list.longest_name as u16 + 4), // +4 for padding
            Constraint::Length(20), // Fixed width for Type
            Constraint::Min(10),    // Remaining space for Value
        ];

        // 5. Construct the Table
        let table = Table::new(rows, widths)
            .header(header)
            .block(block)
            .row_highlight_style(SELECTED_STYLE)
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);

        // 6. Render with State
        StatefulWidget::render(table, area, buf, &mut self.var_list.state);
    }

    fn render_search_footer(&self, area: Rect, buf: &mut Buffer) {

        let search_str = format!("Search: {}", self.search_input);
        let block = Block::new()
            .title(Line::raw(search_str).left_aligned())
            .borders(Borders::TOP)
            .border_set(symbols::border::EMPTY)
            .border_style(TODO_HEADER_STYLE)
            .bg(NORMAL_ROW_BG)
            .padding(Padding::horizontal(1));

        Paragraph::new("".to_string())
            .block(block)
            .fg(TEXT_FG_COLOR)
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }

    fn render_selected_var(&self, area: Rect, buf: &mut Buffer) {

        let (name, desc) = if let Some(var) = self.get_selected_var() {
            let mut values: String = "".to_string();
            if var.var.typ == VarType::Enum {
                values = format!("\n\nPossible values: \n{}", var.var.values.join(", "));
            }
            (var.var.name.clone(), format!("{}{}", var.var.desc, values))
        } else {
            ("No var".to_string(), "Nothing selected...".to_string())
        };

        let block = Block::new()
            .title(Line::raw(name).left_aligned())
            .borders(Borders::TOP)
            .border_set(symbols::border::EMPTY)
            .border_style(TODO_HEADER_STYLE)
            .bg(NORMAL_ROW_BG)
            .padding(Padding::horizontal(1));

        Paragraph::new(desc)
            .block(block)
            .fg(TEXT_FG_COLOR)
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }
}

const fn alternate_colors(i: usize) -> Color {
    if i % 2 == 0 {
        NORMAL_ROW_BG
    } else {
        ALT_ROW_BG_COLOR
    }
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
