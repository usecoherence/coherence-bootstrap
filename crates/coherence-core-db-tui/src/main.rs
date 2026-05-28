mod project_discovery;
mod tree;

use std::env;
use std::path::PathBuf;
use std::process::Command;

use coherence_core_db::db::ConnectionConfig;
use coherence_core_db::models::SpecGraph;
use coherence_core_db::project_manifest;
use coherence_core_db::spec_store;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};

use tree::TreeItem;

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

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    ProjectPicker,
    EnvPicker,
    Specs,
}

#[derive(Clone)]
struct AppState {
    screen: Screen,
    focus_tree: bool,
    edit_mode: bool,
    detail_scroll: u16,
    projects: Vec<(PathBuf, String)>,
    selected_project: usize,
    envs: Vec<String>,
    selected_env: usize,
    graph: Option<SpecGraph>,

    tree_items: Vec<TreeItem>,
    selected_tree: usize,
    detail_spec_id: Option<String>,
    detail_ac_id: Option<String>,

    status: String,
}

impl AppState {
    fn new(projects: Vec<(PathBuf, String)>) -> Self {
        Self {
            screen: Screen::ProjectPicker,
            focus_tree: true,
            edit_mode: false,
            detail_scroll: 0,
            projects,
            selected_project: 0,
            envs: vec!["dev".into(), "test".into(), "prod".into()],
            selected_env: 0,
            graph: None,
            tree_items: Vec::new(),
            selected_tree: 0,
            detail_spec_id: None,
            detail_ac_id: None,
            status: "Select a project".into(),
        }
    }

    fn edit_content(&mut self) {
        let proj_path = self.projects[self.selected_project].0.clone();
        let orig = env::current_dir().ok();
        let _ = env::set_current_dir(&proj_path);

        let config = match ConnectionConfig::from_env() {
            Ok(c) => c,
            Err(_) => {
                if let Some(p) = orig { let _ = env::set_current_dir(p); }
                return;
            }
        };
        let (mut conn, _) = match coherence_core_db::db::connect(&config) {
            Ok(v) => v,
            Err(_) => {
                if let Some(p) = orig { let _ = env::set_current_dir(p); }
                return;
            }
        };

        let (spec_id, ac_id) = (self.detail_spec_id.clone(), self.detail_ac_id.clone());

        if let Some(sid) = spec_id {
            let spec = match spec_store::get_spec(&mut conn, &sid) {
                Ok(Some(s)) => s,
                _ => {
                    if let Some(p) = orig { let _ = env::set_current_dir(p); }
                    return;
                }
            };

            let tmp = format!("/tmp/coherence-spec-{}.md", spec.id);
            if std::fs::write(&tmp, &spec.description).is_err() {
                self.status = "write failed".into();
                if let Some(p) = orig { let _ = env::set_current_dir(p); }
                return;
            }

            let editor = env::var("EDITOR").or_else(|_| env::var("VISUAL")).unwrap_or_else(|_| "micro".to_string());
            let ok = Command::new(&editor).arg(&tmp).status().map(|s| s.success()).unwrap_or(false);

            if ok {
                let new_desc = std::fs::read_to_string(&tmp).unwrap_or_default();
                let mut updated = spec.clone();
                updated.description = new_desc;
                match spec_store::put_spec(&mut conn, &updated) {
                    Ok(()) => {
                        self.status = "Spec description updated".into();
                        if let Some(ref graph) = self.graph {
                            let mut g = graph.clone();
                            if let Some(s) = g.specs.iter_mut().find(|s| s.id == updated.id) {
                                s.description = updated.description.clone();
                            }
                            self.graph = Some(g);
                        }
                    }
                    Err(e) => self.status = format!("update failed: {e}"),
                }
            } else {
                self.status = "Edit cancelled".into();
            }
            let _ = std::fs::remove_file(&tmp);
        } else if let Some(aid) = ac_id {
            let ac = match spec_store::get_acceptance_criterion(&mut conn, &aid) {
                Ok(Some(a)) => a,
                _ => {
                    if let Some(p) = orig { let _ = env::set_current_dir(p); }
                    return;
                }
            };

            let tmp = format!("/tmp/coherence-ac-{}.md", ac.id);
            if std::fs::write(&tmp, &ac.intent).is_err() {
                self.status = "write failed".into();
                if let Some(p) = orig { let _ = env::set_current_dir(p); }
                return;
            }

            let editor = env::var("EDITOR").or_else(|_| env::var("VISUAL")).unwrap_or_else(|_| "micro".to_string());
            let ok = Command::new(&editor).arg(&tmp).status().map(|s| s.success()).unwrap_or(false);

            if ok {
                let new_intent = std::fs::read_to_string(&tmp).unwrap_or_default();
                let mut updated = ac.clone();
                updated.intent = new_intent;
                match spec_store::put_acceptance_criterion(&mut conn, &updated) {
                    Ok(()) => {
                        self.status = "AC intent updated".into();
                        if let Some(ref graph) = self.graph {
                            let mut g = graph.clone();
                            if let Some(a) = g.acceptance_criteria.iter_mut().find(|a| a.id == updated.id) {
                                a.intent = updated.intent.clone();
                            }
                            self.graph = Some(g);
                        }
                    }
                    Err(e) => self.status = format!("update failed: {e}"),
                }
            } else {
                self.status = "Edit cancelled".into();
            }
            let _ = std::fs::remove_file(&tmp);
        }

        if let Some(p) = orig { let _ = env::set_current_dir(p); }
    }

    fn cycle_status(&mut self) {
        let sid = match self.detail_spec_id.clone() { Some(id) => id, None => { self.status = "No spec selected".into(); return; } };
        let proj_path = self.projects[self.selected_project].0.clone();
        let orig = env::current_dir().ok();
        let _ = env::set_current_dir(&proj_path);
        let config = ConnectionConfig::from_env().ok();
        let conn = config.and_then(|c| coherence_core_db::db::connect(&c).ok());
        let (mut conn, _) = match conn { Some(v) => v, None => { if let Some(p) = orig { let _ = env::set_current_dir(p); } return; } };

        let spec = match spec_store::get_spec(&mut conn, &sid) { Ok(Some(s)) => s, _ => { if let Some(p) = orig { let _ = env::set_current_dir(p); } return; } };
        let next = match spec.status {
            coherence_core_db::models::SpecStatus::Draft => coherence_core_db::models::SpecStatus::Active,
            coherence_core_db::models::SpecStatus::Active => coherence_core_db::models::SpecStatus::Deprecated,
            coherence_core_db::models::SpecStatus::Deprecated => coherence_core_db::models::SpecStatus::Archived,
            coherence_core_db::models::SpecStatus::Archived => coherence_core_db::models::SpecStatus::Draft,
        };
        let mut updated = spec.clone();
        updated.status = next;
        if spec_store::put_spec(&mut conn, &updated).is_ok() {
            self.status = format!("Status → {}", next.as_db_str());
            if let Some(ref graph) = self.graph {
                let mut g = graph.clone();
                if let Some(s) = g.specs.iter_mut().find(|s| s.id == updated.id) { s.status = next; }
                self.graph = Some(g);
            }
        }
        if let Some(p) = orig { let _ = env::set_current_dir(p); }
    }

    fn cycle_level(&mut self) {
        let sid = match self.detail_spec_id.clone() { Some(id) => id, None => { self.status = "No spec selected".into(); return; } };
        let proj_path = self.projects[self.selected_project].0.clone();
        let orig = env::current_dir().ok();
        let _ = env::set_current_dir(&proj_path);
        let config = ConnectionConfig::from_env().ok();
        let conn = config.and_then(|c| coherence_core_db::db::connect(&c).ok());
        let (mut conn, _) = match conn { Some(v) => v, None => { if let Some(p) = orig { let _ = env::set_current_dir(p); } return; } };

        let spec = match spec_store::get_spec(&mut conn, &sid) { Ok(Some(s)) => s, _ => { if let Some(p) = orig { let _ = env::set_current_dir(p); } return; } };
        let next = match spec.level {
            coherence_core_db::models::SpecLevel::Product => coherence_core_db::models::SpecLevel::System,
            coherence_core_db::models::SpecLevel::System => coherence_core_db::models::SpecLevel::Module,
            coherence_core_db::models::SpecLevel::Module => coherence_core_db::models::SpecLevel::Product,
        };
        let mut updated = spec.clone();
        updated.level = next;
        if spec_store::put_spec(&mut conn, &updated).is_ok() {
            self.status = format!("Level → {}", next.as_db_str());
            if let Some(ref graph) = self.graph {
                let mut g = graph.clone();
                if let Some(s) = g.specs.iter_mut().find(|s| s.id == updated.id) { s.level = next; }
                self.graph = Some(g);
            }
        }
        if let Some(p) = orig { let _ = env::set_current_dir(p); }
    }

    fn cycle_review_mode(&mut self) {
        let aid = match self.detail_ac_id.clone() { Some(id) => id, None => { self.status = "No AC selected".into(); return; } };
        let proj_path = self.projects[self.selected_project].0.clone();
        let orig = env::current_dir().ok();
        let _ = env::set_current_dir(&proj_path);
        let config = ConnectionConfig::from_env().ok();
        let conn = config.and_then(|c| coherence_core_db::db::connect(&c).ok());
        let (mut conn, _) = match conn { Some(v) => v, None => { if let Some(p) = orig { let _ = env::set_current_dir(p); } return; } };

        let ac = match spec_store::get_acceptance_criterion(&mut conn, &aid) { Ok(Some(a)) => a, _ => { if let Some(p) = orig { let _ = env::set_current_dir(p); } return; } };
        let next = match ac.review_mode {
            coherence_core_db::models::ReviewMode::Manual => coherence_core_db::models::ReviewMode::Automated,
            coherence_core_db::models::ReviewMode::Automated => coherence_core_db::models::ReviewMode::Hybrid,
            coherence_core_db::models::ReviewMode::Hybrid => coherence_core_db::models::ReviewMode::Manual,
        };
        let mut updated = ac.clone();
        updated.review_mode = next;
        if spec_store::put_acceptance_criterion(&mut conn, &updated).is_ok() {
            self.status = format!("Review → {}", next.as_db_str());
            if let Some(ref graph) = self.graph {
                let mut g = graph.clone();
                if let Some(a) = g.acceptance_criteria.iter_mut().find(|a| a.id == updated.id) { a.review_mode = next; }
                self.graph = Some(g);
            }
        }
        if let Some(p) = orig { let _ = env::set_current_dir(p); }
    }

    fn cycle_risk_level(&mut self) {
        let aid = match self.detail_ac_id.clone() { Some(id) => id, None => { self.status = "No AC selected".into(); return; } };
        let proj_path = self.projects[self.selected_project].0.clone();
        let orig = env::current_dir().ok();
        let _ = env::set_current_dir(&proj_path);
        let config = ConnectionConfig::from_env().ok();
        let conn = config.and_then(|c| coherence_core_db::db::connect(&c).ok());
        let (mut conn, _) = match conn { Some(v) => v, None => { if let Some(p) = orig { let _ = env::set_current_dir(p); } return; } };

        let ac = match spec_store::get_acceptance_criterion(&mut conn, &aid) { Ok(Some(a)) => a, _ => { if let Some(p) = orig { let _ = env::set_current_dir(p); } return; } };
        let next = match ac.risk_level {
            coherence_core_db::models::RiskLevel::Low => coherence_core_db::models::RiskLevel::Medium,
            coherence_core_db::models::RiskLevel::Medium => coherence_core_db::models::RiskLevel::High,
            coherence_core_db::models::RiskLevel::High => coherence_core_db::models::RiskLevel::Critical,
            coherence_core_db::models::RiskLevel::Critical => coherence_core_db::models::RiskLevel::Low,
        };
        let mut updated = ac.clone();
        updated.risk_level = next;
        if spec_store::put_acceptance_criterion(&mut conn, &updated).is_ok() {
            self.status = format!("Risk → {}", next.as_db_str());
            if let Some(ref graph) = self.graph {
                let mut g = graph.clone();
                if let Some(a) = g.acceptance_criteria.iter_mut().find(|a| a.id == updated.id) { a.risk_level = next; }
                self.graph = Some(g);
            }
        }
        if let Some(p) = orig { let _ = env::set_current_dir(p); }
    }

    fn load_graph(&mut self) {
        let proj_path = self.projects[self.selected_project].0.clone();
        let env_str = self.envs[self.selected_env].clone();
        let _coherence_env = match env_str.as_str() {
            "test" => project_manifest::CoherenceEnv::Test,
            "prod" => project_manifest::CoherenceEnv::Prod,
            _ => project_manifest::CoherenceEnv::Dev,
        };

        let orig = env::current_dir().ok();
        let _ = env::set_current_dir(&proj_path);

        let config = match ConnectionConfig::from_env() {
            Ok(c) => c,
            Err(e) => {
                self.status = format!("DB connect failed: {e}");
                if let Some(p) = orig {
                    let _ = env::set_current_dir(p);
                }
                return;
            }
        };

        let (mut conn, _) = match coherence_core_db::db::connect(&config) {
            Ok(v) => v,
            Err(e) => {
                self.status = format!("DB connect failed: {e}");
                if let Some(p) = orig {
                    let _ = env::set_current_dir(p);
                }
                return;
            }
        };

        match spec_store::load_spec_graph(&mut conn) {
            Ok(graph) => {
                self.graph = Some(graph);
                self.build_tree();
                self.status = format!("Loaded specs from {}", proj_path.display());
            }
            Err(e) => {
                self.status = format!("Load failed: {e}");
            }
        }

        if let Some(p) = orig {
            let _ = env::set_current_dir(p);
        }
    }

    fn update_preview(&mut self) {
        let (sid, aid) = tree::update_preview(&self.tree_items, self.selected_tree);
        self.detail_spec_id = sid;
        self.detail_ac_id = aid;
        self.detail_scroll = 0;
    }

    fn build_tree(&mut self) {
        self.selected_tree = 0;
        let Some(ref graph) = self.graph.clone() else {
            self.tree_items.clear();
            return;
        };
        tree::build_tree(&mut self.tree_items, &graph);
    }

    fn toggle_expand(&mut self) {
        let Some(ref graph) = self.graph.clone() else {
            return;
        };
        tree::toggle_expand(&mut self.tree_items, self.selected_tree, &graph);
    }
}

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
        if key.kind != KeyEventKind::Press {
            continue;
        }

        match app.screen {
            Screen::ProjectPicker => match key.code {
                KeyCode::Up => {
                    app.selected_project = app.selected_project.saturating_sub(1);
                }
                KeyCode::Down => {
                    app.selected_project =
                        (app.selected_project + 1).min(app.projects.len() - 1);
                }
                KeyCode::Enter => {
                    app.screen = Screen::EnvPicker;
                    app.status = format!(
                        "Select environment for {}",
                        app.projects[app.selected_project].1
                    );
                }
                KeyCode::Esc => {}
                KeyCode::Char('q') => break,
                _ => {}
            },
            Screen::EnvPicker => match key.code {
                KeyCode::Up => {
                    app.selected_env = app.selected_env.saturating_sub(1);
                }
                KeyCode::Down => {
                    app.selected_env =
                        (app.selected_env + 1).min(app.envs.len() - 1);
                }
                KeyCode::Enter => {
                    app.load_graph();
                    app.focus_tree = true;
                    app.screen = Screen::Specs;
                    app.update_preview();
                }
                KeyCode::Esc => {
                    app.screen = Screen::ProjectPicker;
                }
                KeyCode::Char('q') => break,
                _ => {}
            },
            Screen::Specs => {
                if app.edit_mode {
                    match key.code {
                        KeyCode::Char('e') => app.edit_content(),
                        KeyCode::Char('s') => app.cycle_status(),
                        KeyCode::Char('l') => app.cycle_level(),
                        KeyCode::Char('r') => app.cycle_review_mode(),
                        KeyCode::Char('k') => app.cycle_risk_level(),
                        KeyCode::Esc => {
                            app.edit_mode = false;
                            app.status = "Edit mode closed".into();
                        }
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Up if app.focus_tree => {
                    app.selected_tree = app.selected_tree.saturating_sub(1);
                    app.update_preview();
                }
                KeyCode::Down if app.focus_tree => {
                    app.selected_tree = (app.selected_tree + 1)
                        .min(app.tree_items.len().saturating_sub(1));
                    app.update_preview();
                }
                KeyCode::Enter if app.focus_tree => {
                    let item = &app.tree_items[app.selected_tree];
                    if item.has_children {
                        app.toggle_expand();
                    } else {
                        app.focus_tree = false;
                    }
                }
                KeyCode::Up if !app.focus_tree => {
                    app.detail_scroll = app.detail_scroll.saturating_sub(1);
                }
                KeyCode::Down if !app.focus_tree => {
                    app.detail_scroll = app.detail_scroll.saturating_add(1);
                }
                KeyCode::Enter if !app.focus_tree => {
                    app.focus_tree = true;
                }
                KeyCode::Esc if !app.focus_tree => {
                    app.focus_tree = true;
                }
                KeyCode::Esc if app.focus_tree => {
                    app.screen = Screen::EnvPicker;
                    app.status = format!(
                        "Select environment for {}",
                        app.projects[app.selected_project].1
                    );
                }
                KeyCode::Left if app.focus_tree => {
                    if app.tree_items[app.selected_tree].indent > 0 {
                        let cur_indent = app.tree_items[app.selected_tree].indent;
                        for i in (0..app.selected_tree).rev() {
                            if app.tree_items[i].indent < cur_indent {
                                app.selected_tree = i;
                                app.update_preview();
                                break;
                            }
                        }
                    }
                }
                KeyCode::Char('e') => {
                    app.edit_mode = true;
                    app.status = "Edit mode: [s] status  [l] level  [r] review  [k] risk  [e] content  [Esc] exit".into();
                }
                KeyCode::Char('p') => {
                    app.screen = Screen::ProjectPicker;
                    app.status = "Select a project".into();
                }
                KeyCode::Char('d') => {
                    app.screen = Screen::EnvPicker;
                    app.status = format!(
                        "Select environment for {}",
                        app.projects[app.selected_project].1
                    );
                }
                KeyCode::Char('q') => break,
                _ => {}
            }
        }
    }
}
}
    Ok(())
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
