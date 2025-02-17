pub mod table {
    use ratatui::layout::{Constraint, Rect};
    use ratatui::prelude::{Color, Style};
    use ratatui::widgets::{Block, Borders, Row, Table};
    use ratatui::Frame;

    #[derive(Default)]
    pub struct SmartTable {
        is_checked: bool,
        header: Vec<String>,
        data: Vec<Vec<String>>,
        constraints: Vec<Constraint>,
        selected_row: i32,
        title: String,
    }

    impl SmartTable {
        pub fn new(header: Vec<String>, constraints: Vec<Constraint>) -> Self {
            Self {
                is_checked: true,
                header,
                data: Vec::new(),
                constraints,
                selected_row: 0,
                title: String::new(),
            }
        }

        pub fn set_checked(&mut self, checked: bool) {
            self.is_checked = checked;
        }

        pub fn set_data(&mut self, data: Vec<Vec<String>>) {
            self.data = data;
        }

        pub fn set_title(&mut self, title: String) {
            self.title = title;
        }

        pub fn previous_row(&mut self) {
            self.selected_row = (self.selected_row - 1).max(0);
        }

        pub fn next_row(&mut self) {
            self.selected_row = (self.selected_row + 1).min(self.data.len() as i32 - 1);
        }

        pub fn selected_row(&self) -> usize {
            self.selected_row as usize
        }

        pub fn render(&self, frame: &mut Frame, area: Rect) {
            let mut v = vec![Row::new(self.header.clone()).style(Style::default().fg(Color::Gray))];

            for (i, entry) in self.data.iter().enumerate() {
                v.push(
                    Row::new(entry.clone()).style(if self.selected_row as usize == i {
                        Style::default()
                            .bg(if self.is_checked {
                                Color::LightBlue
                            } else {
                                Color::Gray
                            })
                            .fg(Color::White)
                    } else {
                        Style::default()
                    }),
                );
            }

            if self.selected_row as usize > area.rows().count() - 4
                && self.selected_row - ((area.rows().count() - 4) as i32) >= 0
            {
                v = v[(self.selected_row as usize - (area.rows().count() - 4))..].to_vec();
            }

            let table = Table::new(v, self.constraints.clone())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(self.title.as_ref()),
                )
                .style(Style::default().fg(Color::Black));

            frame.render_widget(table, area);
        }
    }
}
