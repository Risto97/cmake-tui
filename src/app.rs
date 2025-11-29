use std::{collections::HashMap, fmt::format, isize};

use color_eyre::Result;
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Layout, Rect, Flex},
    style::{
        Color, Modifier, Style, Stylize,
        palette::tailwind::{BLUE, GREEN, SLATE},
    },
    symbols,
    text::Line,
    widgets::{
        Block, Borders, HighlightSpacing, Padding, Paragraph, StatefulWidget, 
        Widget, Wrap, Table, Row, Cell, TableState, Clear // <--- Updated imports
    },
};

use crate::cache_parser::{CacheEntry, EntryType, parse_cmake_cache};

const TODO_HEADER_STYLE: Style = Style::new().fg(SLATE.c100).bg(BLUE.c800);
const NORMAL_ROW_BG: Color = SLATE.c950;
const ALT_ROW_BG_COLOR: Color = SLATE.c900;
const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);
const TEXT_FG_COLOR: Color = SLATE.c200;
const COMPLETED_TEXT_FG_COLOR: Color = GREEN.c500;

#[derive(PartialEq)]
enum AppMode {
    Scroll,
    ValueEdit,
    SearchInput,
}

pub struct App {
    should_exit: bool,
    cache_list: CacheEntryList,
    mode: AppMode,
    show_advanced: bool,

    search_input: String,
    cursor_pos: usize,
}

struct CacheEntryList {
    all_items: Vec<CacheEntry>,
    items: Vec<CacheEntry>,
    longest_name: usize,
    state: TableState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Status {
    Todo,
    Completed,
}

impl Default for App {
    fn default() -> Self {
        let vec: Vec<CacheEntry> =
            parse_cmake_cache("/tools/work/x-heep/build/").unwrap_or_default();

        let max_len = vec
            .iter()
            .map(|i| i.name.chars().count())
            .max()
            .unwrap_or(100); // Default fallback width
        
        let cache_list = CacheEntryList {
            all_items: vec,
            items: Vec::new(),
            longest_name: max_len,
            state: TableState::default(),
        };

        Self {
            should_exit: false,
            cache_list: cache_list,
            mode: AppMode::Scroll,
            show_advanced: false,

            search_input: "".to_string(),
            cursor_pos: 0,
        }
    }
}

impl App {
    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.rebuild_visible();
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
            KeyCode::Char('/') => self.search_entry(),
            KeyCode::Char('n') => self.select_next_search_result(),
            _ => {}
        }
    }

    fn rebuild_visible(&mut self) {
        self.cache_list.items = self.cache_list.all_items
            .iter()
            .filter(|x| self.show_advanced || !x.advanced)
            .cloned()
            .collect();
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
                if self.cursor_pos > 0{
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

    // fn select_none(&mut self) {
    //     self.cache_list.state.select(None);
    // }

    fn select_next_search_result(&mut self) {
        if self.mode != AppMode::Scroll { return; }
        if self.search_input.is_empty() { return; }

        let query = self.search_input.to_lowercase();

        // Where do we start searching?
        let start = self.cache_list.state.selected().unwrap_or(0);

        let items = &self.cache_list.items;

        // 1) Search from current+1 → end
        for i in start + 1 .. items.len() {
            if items[i].name.to_lowercase().starts_with(&query) {
                self.cache_list.state.select(Some(i));
                return;
            }
        }

        // 2) Wrap around: search from top → current
        for i in 0 ..= start {
            if items[i].name.to_lowercase().starts_with(&query) {
                self.cache_list.state.select(Some(i));
                return;
            }
        }
    }

    fn toggle_show_advanced(&mut self) {
        self.show_advanced = !self.show_advanced;
        self.rebuild_visible();
    }

    fn select_next(&mut self) {
        if self.mode != AppMode::Scroll {return}
        self.cache_list.state.select_next();
    }
    fn select_previous(&mut self) {
        if self.mode != AppMode::Scroll {return}
        self.cache_list.state.select_previous();
    }

    fn select_first(&mut self) {
        if self.mode != AppMode::Scroll {return}
        self.cache_list.state.select_first();
    }

    fn select_last(&mut self) {
        if self.mode != AppMode::Scroll {return}
        self.cache_list.state.select_last();
    }

    fn search_entry(&mut self) {
        if self.mode != AppMode::Scroll {return}
        self.search_input.clear();
        self.cursor_pos = 0;
        self.mode = AppMode::SearchInput;
    }

    fn cycle_value(&mut self) {
        if self.mode != AppMode::Scroll {return}

        let item: &mut CacheEntry = self.get_selected_item_mut().unwrap(); 

        if item.entry_type == EntryType::Bool {
            item.toggle_bool();
        } else if item.entry_type == EntryType::Enum {
            item.cycle_enum();
        }

    }

    fn edit_value(&mut self) {
        if self.mode == AppMode::ValueEdit {
            self.mode = AppMode::Scroll;

        } else if self.mode == AppMode::Scroll {
            if self.get_selected_item().unwrap().entry_type == EntryType::Bool {
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
        self.render_entry_table(list_area, buf);

        if self.mode != AppMode::SearchInput{
            self.render_selected_item(footer_area, buf);
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

    fn get_selected_item_mut(&mut self) -> Option<&mut CacheEntry> {
        self.cache_list.state.selected().map(|i| {
            &mut self.cache_list.items[i]
        })
    }

    fn get_selected_item(&self) -> Option<&CacheEntry>{
        self.cache_list.state.selected().map(|i| {
            &self.cache_list.items[i]
        })
    }

    fn render_popup(&self, area: Rect, buf: &mut Buffer) {
        if self.mode != AppMode::ValueEdit {return};

        let item = self.get_selected_item().unwrap(); // TODO fix unwrap

        // Format the detailed content. Use Line::from(Vec<Span>) for rich text.
        let content = vec![
            Line::from(format!("Name: {}", item.name)).bold(),
            Line::from(format!("Type: {}", item.entry_type)),
            // Line::from(format!("Value: {}", item.value)),
            // Line::from(vec![
            //     "Description: ".bold(),
            //     // Assuming 'desc' field exists on CacheEntry based on your prior commented code
            //     item.desc.clone().into(), 
            // ]),
        ];
        // let content = vec![Line::from(format!("Name")).bold()];

        // 2. Define the size and position of the popup
        let popup_area = popup_area(area, 20, 10); // 70% width, 50% height
        Clear.render(popup_area, buf);

        // // 3. Define the Block
        let block = Block::new()
            .title(Line::raw("Full Cache Entry Details").centered().bold())
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
    fn render_entry_table(&mut self, area: Rect, buf: &mut Buffer) {
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

        // 3. Define the Rows from Items
        let rows: Vec<Row> = self
            .cache_list
            .items
            .iter()
            // .filter(|item| self.show_advanced || !item.advanced)
            .enumerate()
            .map(|(i, item)| {
                let color = alternate_colors(i);
                
                // Assuming item.name, item.entry_type, item.value implement Display
                Row::new(vec![
                    Cell::from(item.name.clone()),
                    Cell::from(item.entry_type.to_string()), 
                    Cell::from(item.value.to_string()),
                ])
                .style(Style::new().bg(color).fg(TEXT_FG_COLOR))
            })
            .collect();

        // 4. Define Column Widths
        // We use the calculated longest_name for the first column
        let widths = [
            Constraint::Length(self.cache_list.longest_name as u16 + 4), // +4 for padding
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
        StatefulWidget::render(table, area, buf, &mut self.cache_list.state);
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

    fn render_selected_item(&self, area: Rect, buf: &mut Buffer) {

        let (name, desc) = if let Some(item) = self.get_selected_item() {
            let mut values: String = "".to_string();
            if item.entry_type == EntryType::Enum {
                values = format!("\n\nPossible values: \n{}", item.values.join(", "));
            }
            (item.name.clone(), format!("{}{}", item.desc, values))
        } else {
            ("No item".to_string(), "Nothing selected...".to_string())
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
