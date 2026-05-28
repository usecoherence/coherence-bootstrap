mod action;
mod app;
mod effects;
mod project_discovery;
mod tree;
mod update;

use ratatui::crossterm::event::{self, Event};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};

use app::{AppState, Screen};
use action::{key_to_action, AppAction};
use update::update;

struct Theme {
    title_bg: Color,
    title_fg: Color,
    selected_bg: Color,
    selected_fg: Color,
    spec_fg: Color,
    ac_fg: Color,
    level_header_fg: Color,
    border_focused: Color,
    border_inactive: Color,
    status_bg: Color,
    status_fg: Color,
    env_fg: Color,
}

static THEME: Theme = Theme {
    title_bg: Color::Rgb(60, 60, 120),
    title_fg: Color::Rgb(220, 220, 255),
    selected_bg: Color::Rgb(80, 80, 160),
    selected_fg: Color::Rgb(255, 255, 255),
    spec_fg: Color::Rgb(150, 200, 255),
    ac_fg: Color::Rgb(180, 180, 180),
    level_header_fg: Color::Rgb(255, 200, 100),
    border_focused: Color::Rgb(255, 200, 100),
    border_inactive: Color::Rgb(80, 80, 80),
    status_bg: Color::Rgb(30, 30, 50),
    status_fg: Color::Rgb(180, 180, 220),
    env_fg: Color::Rgb(200, 200, 100),
};

fn main() -> Result<(), String> {
    let projects = project_discovery::discover_projects();
    if projects.is_empty() {
        eprintln!("No Coherence projects found under ~/git/");
        eprintln!("(Looking for ~/git/**/*/.coherence/project.toml with find -maxdepth 6)");
        eprintln!("Try: find ~/git -name project.toml -path '*/.coherence/project.toml'");
        return Ok(());
    }

    let mut app = AppState::new(projects);
    let terminal = ratatui::init();
    let result = run(terminal, &mut app);
    ratatui::restore();
    result
}

fn run(mut terminal: DefaultTerminal, app: &mut AppState) -> Result<(), String> {
    loop {
        terminal
            .draw(|frame| ui(frame, app))
            .map_err(|e| format!("draw: {e}"))?;

        if !event::poll(std::time::Duration::from_millis(100))
            .map_err(|e| format!("poll: {e}"))?
        {
            continue;
        }

        let Event::Key(key) = event::read().map_err(|e| format!("read: {e}"))? else {
            continue;
        };

        let Some(action) = key_to_action(key, app) else {
            continue;
        };

        if matches!(action, AppAction::Quit) {
            break Ok(());
        }

        let effects = update(app, action);
        effects::execute_effects(app, effects);
    }
}

fn title_line(app: &AppState) -> Line<'static> {
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
            .fg(THEME.title_fg)
            .bg(THEME.title_bg)
            .add_modifier(Modifier::BOLD),
    ))
}

fn ui(frame: &mut Frame, app: &AppState) {
    let [title_area, main_area, status_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    frame.render_widget(Paragraph::new(title_line(app)), title_area);

    match app.screen {
        Screen::ProjectPicker => render_project_picker(frame, main_area, app),
        Screen::EnvPicker => render_env_picker(frame, main_area, app),
        Screen::Specs => {
            if app.edit_mode {
                render_detail(frame, main_area, app);
            } else {
                let [left, right] = Layout::horizontal([
                    Constraint::Percentage(40),
                    Constraint::Percentage(60),
                ])
                .areas(main_area);
                render_tree(frame, left, app);
                render_detail(frame, right, app);
            }
        }
    }

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            &app.status,
            Style::default()
                .fg(THEME.status_fg)
                .bg(THEME.status_bg),
        ))),
        status_area,
    );
}

fn render_project_picker(frame: &mut Frame, area: Rect, app: &AppState) {
    let items: Vec<ListItem> = app
        .projects
        .iter()
        .enumerate()
        .map(|(i, (path, slug))| {
            let style = if i == app.selected_project {
                Style::default()
                    .bg(THEME.selected_bg)
                    .fg(THEME.selected_fg)
            } else {
                Style::default().fg(THEME.env_fg)
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

fn render_env_picker(frame: &mut Frame, area: Rect, app: &AppState) {
    let items: Vec<ListItem> = app
        .envs
        .iter()
        .enumerate()
        .map(|(i, env)| {
            let style = if i == app.selected_env {
                Style::default()
                    .bg(THEME.selected_bg)
                    .fg(THEME.selected_fg)
            } else {
                Style::default().fg(THEME.env_fg)
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

fn render_tree(frame: &mut Frame, area: Rect, app: &AppState) {
    let border_color = if app.focus_tree {
        THEME.border_focused
    } else {
        THEME.border_inactive
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
                THEME.level_header_fg
            } else if item.is_spec {
                THEME.spec_fg
            } else {
                THEME.ac_fg
            };

            let style = if i == app.selected_tree {
                Style::default()
                    .bg(THEME.selected_bg)
                    .fg(THEME.selected_fg)
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

fn render_detail(frame: &mut Frame, area: Rect, app: &AppState) {
    let border_color = if app.edit_mode {
        Color::Rgb(100, 220, 100)
    } else if app.focus_tree {
        THEME.border_inactive
    } else {
        THEME.border_focused
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
