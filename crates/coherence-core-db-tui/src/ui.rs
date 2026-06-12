use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use coherence_core_db::ac_verify::AcVerifyOverallStatus;
use coherence_core_db::models::{AcceptanceCriterion, Spec, SpecGraph};

use crate::app::{AppState, Screen};
use crate::edit::Draft;
use crate::theme::Theme;

pub fn ui(frame: &mut Frame, app: &mut AppState, theme: &Theme) {
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
                let [left, right] =
                    Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
                        .areas(main_area);
                render_tree(frame, left, app, theme);
                render_detail(frame, right, app, theme);
            }
        }
    }

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            &app.status,
            Style::default().fg(theme.status_fg).bg(theme.status_bg),
        ))),
        status_area,
    );
}

fn title_hint(app: &AppState) -> &'static str {
    match app.screen {
        Screen::ProjectPicker => " [↑↓] nav  [Enter] select  [q] quit",
        Screen::EnvPicker => " [↑↓] nav  [Enter] select  [Esc] back  [q] quit",
        Screen::Specs if app.edit_mode => {
            " [s] status  [l] level  [r] review  [k] risk  [e] editor  [Enter] save  [Esc] cancel"
        }
        Screen::Specs if app.focus_tree => {
            " [↑↓] nav  [Enter] expand/open  [v] verify  [V] all  [p] project  [d] DB  [e] edit  [q] quit"
        }
        Screen::Specs => {
            " [↑↓] scroll  [Enter/Esc] back to tree  [v] verify  [V] all  [e] edit  [q] quit"
        }
    }
}

fn title_label(app: &AppState) -> &'static str {
    match app.screen {
        Screen::ProjectPicker => " Coherence Spec Browser  ",
        Screen::EnvPicker => " Environment  ",
        Screen::Specs if app.focus_tree => " Specs  ",
        Screen::Specs => " Detail  ",
    }
}

fn title_line(app: &AppState, theme: &Theme) -> Line<'static> {
    Line::from(Span::styled(
        format!("{}{}", title_label(app), title_hint(app)),
        Style::default()
            .fg(theme.title_fg)
            .bg(theme.title_bg)
            .add_modifier(Modifier::BOLD),
    ))
}

fn render_picker(frame: &mut Frame, area: Rect, items: Vec<ListItem>, title: &str) {
    frame.render_widget(
        List::new(items).block(Block::default().borders(Borders::ALL).title(title)),
        area,
    );
}

fn render_project_picker(frame: &mut Frame, area: Rect, app: &AppState, theme: &Theme) {
    let items = app
        .projects
        .iter()
        .enumerate()
        .map(|(i, (path, slug))| {
            let style = if i == app.selected_project {
                Style::default().bg(theme.selected_bg).fg(theme.selected_fg)
            } else {
                Style::default().fg(theme.env_fg)
            };
            ListItem::new(Line::from(Span::styled(
                format!(" {}  ({})", slug, path.display()),
                style,
            )))
        })
        .collect();
    render_picker(frame, area, items, " Projects ");
}

fn render_env_picker(frame: &mut Frame, area: Rect, app: &AppState, theme: &Theme) {
    let items = app
        .envs
        .iter()
        .enumerate()
        .map(|(i, env)| {
            let style = if i == app.selected_env {
                Style::default().bg(theme.selected_bg).fg(theme.selected_fg)
            } else {
                Style::default().fg(theme.env_fg)
            };
            ListItem::new(Line::from(Span::styled(format!(" {env} "), style)))
        })
        .collect();
    render_picker(frame, area, items, " Environment ");
}

fn render_tree(frame: &mut Frame, area: Rect, app: &mut AppState, theme: &Theme) {
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

    let viewport_height = area.height.saturating_sub(2) as usize;
    app.ensure_tree_selection_visible(viewport_height);
    let visible_start = app.tree_scroll.min(app.tree_items.len());
    let visible_end = visible_start
        .saturating_add(viewport_height)
        .min(app.tree_items.len());

    let items = build_tree_items(app, theme, visible_start, visible_end);

    let title = tree_title(title, app, viewport_height);

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

fn build_tree_items<'a>(
    app: &'a AppState,
    theme: &'a Theme,
    start: usize,
    end: usize,
) -> Vec<ListItem<'a>> {
    app.tree_items
        .iter()
        .enumerate()
        .skip(start)
        .take(end.saturating_sub(start))
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
            let marker = item.parent_spec_id.as_ref().map_or_else(String::new, |_| {
                format!(
                    "{} ",
                    verification_marker(app.verification_statuses.get(&item.id).copied())
                )
            });
            let label = format!("{}{marker}{prefix}{}", indent, item.label);

            let fg = if item.indent == 0 {
                theme.level_header_fg
            } else if item.is_spec {
                theme.spec_fg
            } else {
                theme.ac_fg
            };

            let style = if i == app.selected_tree {
                Style::default().bg(theme.selected_bg).fg(theme.selected_fg)
            } else {
                Style::default().fg(fg)
            };
            ListItem::new(Line::from(Span::styled(label, style)))
        })
        .collect()
}

fn tree_title(title: &str, app: &AppState, viewport_height: usize) -> String {
    if app.tree_items.len() > viewport_height && viewport_height > 0 {
        format!(
            "{title} {}/{} ",
            app.selected_tree.saturating_add(1),
            app.tree_items.len()
        )
    } else {
        title.to_string()
    }
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

    let content = detail_content(app, graph);

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

fn find_draft<'a>(app: &'a AppState, id: &str) -> Option<&'a Draft> {
    app.draft.as_ref().and_then(|d| match d {
        Draft::Spec { spec_id, .. } if spec_id == id => Some(d),
        Draft::Ac { ac_id, .. } if ac_id == id => Some(d),
        _ => None,
    })
}

fn detail_content(app: &AppState, graph: &SpecGraph) -> String {
    let dirty = app.draft.as_ref().is_some_and(Draft::is_dirty);
    let changes = if dirty { "  [modified]" } else { "" };

    if let Some(ref spec_id) = app.detail_spec_id {
        return detail_spec_or_not_found(app, graph, spec_id, changes);
    }
    if let Some(ref ac_id) = app.detail_ac_id {
        return detail_ac_or_not_found(app, graph, ac_id);
    }
    "Select a spec or AC from the tree".to_string()
}

fn detail_spec_or_not_found(
    app: &AppState,
    graph: &SpecGraph,
    spec_id: &str,
    changes: &str,
) -> String {
    match graph.specs.iter().find(|s| s.id == *spec_id) {
        Some(s) => detail_spec_content(s, find_draft(app, spec_id), graph, changes),
        None => "Spec not found".into(),
    }
}

fn detail_ac_or_not_found(app: &AppState, graph: &SpecGraph, ac_id: &str) -> String {
    match graph.acceptance_criteria.iter().find(|a| a.id == *ac_id) {
        Some(a) => detail_ac_content(a, find_draft(app, ac_id), app),
        None => "AC not found".into(),
    }
}

fn detail_spec_content(
    s: &Spec,
    draft: Option<&Draft>,
    graph: &SpecGraph,
    changes: &str,
) -> String {
    let (level, status, desc) = match draft {
        Some(Draft::Spec {
            pending_level,
            pending_status,
            pending_description,
            ..
        }) => {
            let d = pending_description.as_deref().unwrap_or(&s.description);
            (pending_level.as_db_str(), pending_status.as_db_str(), d)
        }
        _ => (
            s.level.as_db_str(),
            s.status.as_db_str(),
            &s.description as &str,
        ),
    };
    let concerns = graph
        .acceptance_criteria
        .iter()
        .filter(|a| a.spec_id == s.id)
        .map(|a| {
            format!(
                "      ├ {} — {} (risk={} review={})",
                a.slug,
                a.title,
                a.risk_level.as_db_str(),
                a.review_mode.as_db_str()
            )
        })
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
         ┃  Description:{changes}    \n\
         {}\n\
         {}\n\
         ┗{:━^80}┛",
        " Spec ",
        s.id,
        s.slug,
        s.title,
        level,
        status,
        "",
        textwrap_indent(desc, "┃  "),
        if concerns.is_empty() {
            String::new()
        } else {
            format!("┃\n┃  Acceptance Criteria:\n{concerns}\n")
        },
        ""
    )
}

fn detail_ac_content(a: &AcceptanceCriterion, draft: Option<&Draft>, app: &AppState) -> String {
    let (review_mode, risk_level, intent_changes, intent) = match draft {
        Some(Draft::Ac {
            pending_review_mode,
            pending_risk_level,
            pending_intent,
            ..
        }) => {
            let i = pending_intent.as_deref().unwrap_or(&a.intent);
            let ch = if pending_intent.is_some() {
                "  [modified]"
            } else {
                ""
            };
            (
                pending_review_mode.as_db_str(),
                pending_risk_level.as_db_str(),
                ch,
                i,
            )
        }
        _ => (
            a.review_mode.as_db_str(),
            a.risk_level.as_db_str(),
            "",
            &a.intent as &str,
        ),
    };
    let concerns_str = a
        .concerns
        .iter()
        .map(|c| c.as_db_str())
        .collect::<Vec<_>>()
        .join(", ");
    let verification = verification_description(app.verification_statuses.get(&a.id).copied());
    format!(
        "┌ {:━^78}┐\n\
         ┃  ID         │ {}    \n\
         ┃  Spec ID    │ {}    \n\
         ┃  Slug       │ {}    \n\
         ┃  Title      │ {}    \n\
         ┃  Review     │ {}    \n\
         ┃  Risk       │ {}    \n\
         ┃  Concerns   │ {}    \n\
         ┃  Verify     │ {}    \n\
         ┣{:━^80}┨\n\
         ┃  Intent:{intent_changes}    \n\
         {}\n\
         ┗{:━^80}┛",
        " Acceptance Criterion ",
        a.id,
        a.spec_id,
        a.slug,
        a.title,
        review_mode,
        risk_level,
        concerns_str,
        verification,
        "",
        textwrap_indent(intent, "┃  "),
        ""
    )
}

fn textwrap_indent(text: &str, prefix: &str) -> String {
    text.lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn verification_marker(status: Option<AcVerifyOverallStatus>) -> &'static str {
    match status {
        Some(AcVerifyOverallStatus::Passed) => "[+]",
        Some(AcVerifyOverallStatus::Failed) => "[!]",
        Some(AcVerifyOverallStatus::Skipped) => "[?]",
        Some(AcVerifyOverallStatus::NoVerification) => "[-]",
        None => "[ ]",
    }
}

fn verification_description(status: Option<AcVerifyOverallStatus>) -> String {
    match status {
        Some(AcVerifyOverallStatus::Passed) => "[+] passed".to_string(),
        Some(AcVerifyOverallStatus::Failed) => "[!] failed".to_string(),
        Some(AcVerifyOverallStatus::Skipped) => {
            "[?] linked test missing/skipped or not materialized".to_string()
        }
        Some(AcVerifyOverallStatus::NoVerification) => "[-] no verified_by link".to_string(),
        None => "[ ] never run".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verification_marker_distinguishes_all_ac_states() {
        assert_eq!(
            verification_marker(Some(AcVerifyOverallStatus::Passed)),
            "[+]"
        );
        assert_eq!(
            verification_marker(Some(AcVerifyOverallStatus::Failed)),
            "[!]"
        );
        assert_eq!(
            verification_marker(Some(AcVerifyOverallStatus::Skipped)),
            "[?]"
        );
        assert_eq!(
            verification_marker(Some(AcVerifyOverallStatus::NoVerification)),
            "[-]"
        );
        assert_eq!(verification_marker(None), "[ ]");
    }

    #[test]
    fn verification_description_explains_not_run_and_missing_links() {
        assert!(verification_description(None).contains("never run"));
        assert!(
            verification_description(Some(AcVerifyOverallStatus::NoVerification))
                .contains("no verified_by")
        );
        assert!(
            verification_description(Some(AcVerifyOverallStatus::Skipped))
                .contains("missing/skipped")
        );
    }
}
