use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{AppState, Screen};
use crate::theme::Theme;

pub fn ui(frame: &mut Frame, app: &AppState, theme: &Theme) {
    let [title_area, main_area, status_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    frame.render_widget(Paragraph::new(title_line(app, theme)), title_area);

    match app.screen {
        Screen::ProjectPicker => render_project_picker(frame, main_area, app, theme),
        Screen::EnvPicker => render_env_picker(frame, main_area, app, theme),
        Screen::Specs => {
            if app.edit_mode {
                render_detail(frame, main_area, app, theme);
            } else {
                let [left, right] = Layout::horizontal([
                    Constraint::Percentage(40),
                    Constraint::Percentage(60),
                ])
                .areas(main_area);
                render_tree(frame, left, app, theme);
                render_detail(frame, right, app, theme);
            }
        }
    }

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            &app.status,
            Style::default()
                .fg(theme.status_fg)
                .bg(theme.status_bg),
        ))),
        status_area,
    );
}

fn title_line(app: &AppState, theme: &Theme) -> Line<'static> {
    let hint = match app.screen {
        Screen::ProjectPicker => " [↑↓] nav  [Enter] select  [q] quit",
        Screen::EnvPicker => " [↑↓] nav  [Enter] select  [Esc] back  [q] quit",
        Screen::Specs => {
            if app.edit_mode {
                " [s] status  [l] level  [r] review  [k] risk  [e] open editor  [Esc] done"
            } else if app.focus_tree {
                " [↑↓] nav  [Enter] expand/open  [p] project  [d] DB  [e] edit  [Esc] back  [q] quit"
            } else {
                " [↑↓] scroll  [Enter/Esc] back to tree  [e] edit  [p] project  [d] DB  [q] quit"
            }
        }
    };
    let label = match app.screen {
        Screen::ProjectPicker => " Coherence Spec Browser  ",
        Screen::EnvPicker => " Environment  ",
        Screen::Specs => {
            if app.focus_tree {
                " Specs  "
            } else {
                " Detail  "
            }
        }
    };
    Line::from(Span::styled(
        format!("{}{}", label, hint),
        Style::default()
            .fg(theme.title_fg)
            .bg(theme.title_bg)
            .add_modifier(Modifier::BOLD),
    ))
}

fn render_project_picker(frame: &mut Frame, area: Rect, app: &AppState, theme: &Theme) {
    let items: Vec<ListItem> = app
        .projects
        .iter()
        .enumerate()
        .map(|(i, (path, slug))| {
            let style = if i == app.selected_project {
                Style::default()
                    .bg(theme.selected_bg)
                    .fg(theme.selected_fg)
            } else {
                Style::default().fg(theme.env_fg)
            };
            ListItem::new(Line::from(Span::styled(
                format!(" {}  ({})", slug, path.display()),
                style,
            )))
        })
        .collect();

    frame.render_widget(
        List::new(items)
            .block(Block::default().borders(Borders::ALL).title(" Projects ")),
        area,
    );
}

fn render_env_picker(frame: &mut Frame, area: Rect, app: &AppState, theme: &Theme) {
    let items: Vec<ListItem> = app
        .envs
        .iter()
        .enumerate()
        .map(|(i, env)| {
            let style = if i == app.selected_env {
                Style::default()
                    .bg(theme.selected_bg)
                    .fg(theme.selected_fg)
            } else {
                Style::default().fg(theme.env_fg)
            };
            ListItem::new(Line::from(Span::styled(format!(" {env} "), style)))
        })
        .collect();

    frame.render_widget(
        List::new(items)
            .block(Block::default().borders(Borders::ALL).title(" Environment ")),
        area,
    );
}

fn render_tree(frame: &mut Frame, area: Rect, app: &AppState, theme: &Theme) {
    let border_color = if app.focus_tree {
        theme.border_focused
    } else {
        theme.border_inactive
    };

    let title = if app.focus_tree {
        " Specs "
    } else {
        " Specs (inactive) "
    };

    let items: Vec<ListItem> = app
        .tree_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let prefix = if item.has_children {
                if item.expanded {
                    "▼ "
                } else {
                    "▶ "
                }
            } else {
                "  "
            };
            let indent = "  ".repeat(item.indent);
            let label = format!("{}{prefix}{}", indent, item.label);

            let fg = if item.indent == 0 {
                theme.level_header_fg
            } else if item.is_spec {
                theme.spec_fg
            } else {
                theme.ac_fg
            };

            let style = if i == app.selected_tree {
                Style::default()
                    .bg(theme.selected_bg)
                    .fg(theme.selected_fg)
            } else {
                Style::default().fg(fg)
            };
            ListItem::new(Line::from(Span::styled(label, style)))
        })
        .collect();

    frame.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(title),
        ),
        area,
    );
}

fn render_detail(frame: &mut Frame, area: Rect, app: &AppState, theme: &Theme) {
    let border_color = if app.edit_mode {
        Color::Rgb(100, 220, 100)
    } else if app.focus_tree {
        theme.border_inactive
    } else {
        theme.border_focused
    };

    let title = if app.edit_mode {
        " Edit "
    } else if app.focus_tree {
        " Detail (preview) "
    } else {
        " Detail "
    };

    let Some(ref graph) = app.graph else {
        return;
    };

    let content = if let Some(ref spec_id) = app.detail_spec_id {
        let spec = graph.specs.iter().find(|s| s.id == *spec_id);
        match spec {
            Some(s) => {
                let concerns = graph.acceptance_criteria.iter()
                    .filter(|a| a.spec_id == s.id)
                    .map(|a| format!("      ├ {} — {} (risk={} review={})", a.slug, a.title, a.risk_level.as_db_str(), a.review_mode.as_db_str()))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!(
                    "┌ {:━^78}┐\n\
                     ┃  ID       │ {}    \n\
                     ┃  Slug     │ {}    \n\
                     ┃  Title    │ {}    \n\
                     ┃  Level    │ {}    \n\
                     ┃  Status   │ {}    \n\
                     ┣{:━^80}┨\n\
                      ┃  Description:    \n\
                      {}\n\
                     {}\n\
                     ┗{:━^80}┛",
                    " Spec ", s.id, s.slug, s.title,
                    s.level.as_db_str(), s.status.as_db_str(), "",
                    textwrap_indent(&s.description, "┃  "),
                    if concerns.is_empty() { String::new() } else { format!("┃\n┃  Acceptance Criteria:\n{concerns}\n") },
                    ""
                )
            }
            None => "Spec not found".into(),
        }
    } else if let Some(ref ac_id) = app.detail_ac_id {
        let ac = graph
            .acceptance_criteria
            .iter()
            .find(|a| a.id == *ac_id);
        match ac {
            Some(a) => {
                let concerns_str = a.concerns.iter()
                    .map(|c| c.as_db_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "┌ {:━^78}┐\n\
                     ┃  ID         │ {}    \n\
                     ┃  Spec ID    │ {}    \n\
                     ┃  Slug       │ {}    \n\
                     ┃  Title      │ {}    \n\
                     ┃  Review     │ {}    \n\
                     ┃  Risk       │ {}    \n\
                     ┃  Concerns   │ {}    \n\
                     ┣{:━^80}┨\n\
                     ┃  Intent:    \n\
                     {}\n\
                     ┗{:━^80}┛",
                    " Acceptance Criterion ", a.id, a.spec_id, a.slug, a.title,
                    a.review_mode.as_db_str(), a.risk_level.as_db_str(), concerns_str, "",
                    textwrap_indent(&a.intent, "┃  "),
                    ""
                )
            }
            None => "AC not found".into(),
        }
    } else {
        "Select a spec or AC from the tree".to_string()
    };

    frame.render_widget(
        Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .title(title),
            )
            .scroll((app.detail_scroll, 0))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn textwrap_indent(text: &str, prefix: &str) -> String {
    text.lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}
