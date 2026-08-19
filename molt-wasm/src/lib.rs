//! Browser terminal components for Molt.

// Re-export molt_forked so application command macros remain hygienic through this crate.
use molt::prelude::*;
use molt::syntax::{self, ParseStatus, SyntaxAnalysis, SyntaxKind};
pub use molt_forked as molt;
use std::{mem, rc::Rc};
use web_sys::HtmlTextAreaElement;
use yew::prelude::*;
use yew_icons::{Icon, IconData};

/// Whether a submitted script succeeded.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub enum RunState {
    /// The script completed normally.
    #[default]
    Ok,
    /// The script returned an error.
    Err,
}

/// An immutable terminal history entry with cached syntax information.
#[derive(Debug, PartialEq, Clone)]
pub struct TerminalEntry {
    state: RunState,
    source: String,
    output: Html,
    analysis: SyntaxAnalysis,
}

impl TerminalEntry {
    /// Creates an entry and analyzes its source exactly once.
    #[must_use]
    pub fn new(state: RunState, source: String, output: Html) -> Self {
        let analysis = syntax::analyze_script(&source);
        Self { state, source, output, analysis }
    }

    /// Returns the execution state.
    #[must_use]
    pub const fn state(&self) -> RunState {
        self.state
    }

    /// Returns the exact submitted Tcl source.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Returns the rendered command output.
    pub fn output(&self) -> &Html {
        &self.output
    }
}

/// Properties for [`Terminal`].
#[derive(Debug, Properties, PartialEq)]
pub struct TerminalProps {
    /// CSS classes applied to the terminal root.
    #[prop_or_default]
    pub class: Classes,
    /// Previously submitted scripts and their output.
    pub entries: Rc<Vec<TerminalEntry>>,
    /// Called only with complete or invalid-but-complete source.
    pub on_submit: Callback<String>,
}

/// A Tcl-aware terminal input and history view.
pub struct Terminal {
    input_ref: NodeRef,
    highlight_ref: NodeRef,
    history_ref: NodeRef,
    input: String,
    analysis: SyntaxAnalysis,
    input_before_history: String,
    current_history: Option<usize>,
    move_cursor_to_end: bool,
    scroll_history_to_end: bool,
}

/// Internal component messages.
pub enum TerminalMsg {
    UpdateInput(String),
    Submit,
    HistoryPrevious,
    HistoryNext,
    None,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum EnterAction {
    InsertNewline,
    Submit,
    Ignore,
}

fn enter_action(status: ParseStatus, shift: bool, composing: bool) -> EnterAction {
    if composing {
        EnterAction::Ignore
    } else if shift || status.is_incomplete() {
        EnterAction::InsertNewline
    } else {
        EnterAction::Submit
    }
}

impl Terminal {
    /// Converts interpreter results into a typed, syntax-cached history entry.
    #[must_use]
    pub fn to_entry(
        source: String,
        outputs: Vec<Result<Value, Exception>>,
    ) -> TerminalEntry {
        let state =
            if outputs.iter().any(Result::is_err) { RunState::Err } else { RunState::Ok };
        let output = html! {
            {for outputs.iter().enumerate().map(|(index, result)| {
                let separator = (index + 1 < outputs.len()).then(|| html!(<br />));
                match result {
                    Ok(value) => html!(
                        <code class="stdout">{value.to_string()}{separator}</code>
                    ),
                    Err(error) => html!(
                        <code class="stderr">{error.error_info().to_string()}{separator}</code>
                    ),
                }
            })}
        };
        TerminalEntry::new(state, source, output)
    }

    fn set_input(&mut self, input: String) {
        self.analysis = syntax::analyze_script(&input);
        self.input = input;
    }

    fn request_cursor_at_end(&mut self) {
        self.move_cursor_to_end = true;
    }

    fn request_history_at_end(&mut self) {
        self.scroll_history_to_end = true;
    }

    fn previous_history(&mut self, entries: &[TerminalEntry]) -> bool {
        let Some(last) = entries.len().checked_sub(1) else {
            return false;
        };
        let index = match self.current_history {
            Some(0) => return false,
            Some(index) => index - 1,
            None => {
                self.input_before_history = self.input.clone();
                last
            }
        };
        self.current_history = Some(index);
        self.set_input(entries[index].source.clone());
        self.request_cursor_at_end();
        true
    }

    fn next_history(&mut self, entries: &[TerminalEntry]) -> bool {
        let Some(index) = self.current_history else {
            return false;
        };
        if index + 1 < entries.len() {
            let next = index + 1;
            self.current_history = Some(next);
            self.set_input(entries[next].source.clone());
        } else {
            self.current_history = None;
            let input = mem::take(&mut self.input_before_history);
            self.set_input(input);
        }
        self.request_cursor_at_end();
        true
    }
}

impl Component for Terminal {
    type Message = TerminalMsg;
    type Properties = TerminalProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            input_ref: NodeRef::default(),
            highlight_ref: NodeRef::default(),
            history_ref: NodeRef::default(),
            input: String::new(),
            analysis: syntax::analyze_script(""),
            input_before_history: String::new(),
            current_history: None,
            move_cursor_to_end: false,
            scroll_history_to_end: true,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, message: Self::Message) -> bool {
        match message {
            TerminalMsg::UpdateInput(input) => {
                self.set_input(input);
                self.current_history = None;
                self.input_before_history.clear();
                true
            }
            TerminalMsg::Submit => {
                if self.input.trim().is_empty() {
                    return false;
                }
                let source = mem::take(&mut self.input);
                self.analysis = syntax::analyze_script("");
                self.current_history = None;
                self.input_before_history.clear();
                ctx.props().on_submit.emit(source);
                true
            }
            TerminalMsg::HistoryPrevious => self.previous_history(&ctx.props().entries),
            TerminalMsg::HistoryNext => self.next_history(&ctx.props().entries),
            TerminalMsg::None => false,
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, old_props: &Self::Properties) -> bool {
        if ctx.props().entries.len() > old_props.entries.len() {
            self.request_history_at_end();
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let highlight_ref = self.highlight_ref.clone();
        let status = self.analysis.status();
        html! {
            <div class={classes!("molt-terminal", ctx.props().class.clone())}>
                <ul ref={self.history_ref.clone()} class="history">
                    {for ctx.props().entries.iter().map(render_history_entry)}
                </ul>
                <div class="molt-input-layer">
                    <pre
                        ref={self.highlight_ref.clone()}
                        class="input-highlight"
                        aria-hidden="true"
                    >{render_source(&self.input, &self.analysis)}</pre>
                    <textarea
                        ref={self.input_ref.clone()}
                        class="input"
                        value={self.input.clone()}
                        aria-label="Tcl command input"
                        autocomplete="off"
                        autocapitalize="off"
                        spellcheck="false"
                        wrap="off"
                        oninput={ctx.link().callback(|event: InputEvent| {
                            let input: HtmlTextAreaElement = event.target_unchecked_into();
                            TerminalMsg::UpdateInput(input.value())
                        })}
                        onscroll={Callback::from(move |event: Event| {
                            let input: HtmlTextAreaElement = event.target_unchecked_into();
                            if let Some(highlight) = highlight_ref.cast::<web_sys::Element>() {
                                highlight.set_scroll_top(input.scroll_top());
                                highlight.set_scroll_left(input.scroll_left());
                            }
                        })}
                        onkeydown={ctx.link().callback(move |event: KeyboardEvent| {
                            match event.key().as_str() {
                                "Enter" => match enter_action(
                                    status,
                                    event.shift_key(),
                                    event.is_composing(),
                                ) {
                                    EnterAction::Submit => {
                                        event.prevent_default();
                                        TerminalMsg::Submit
                                    }
                                    EnterAction::InsertNewline | EnterAction::Ignore => {
                                        TerminalMsg::None
                                    }
                                },
                                "ArrowUp" if !event.alt_key() && !event.meta_key() => {
                                    event.prevent_default();
                                    TerminalMsg::HistoryPrevious
                                }
                                "ArrowDown" if !event.alt_key() && !event.meta_key() => {
                                    event.prevent_default();
                                    TerminalMsg::HistoryNext
                                }
                                _ => TerminalMsg::None,
                            }
                        })}
                    />
                </div>
            </div>
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, _first_render: bool) {
        if mem::take(&mut self.move_cursor_to_end) {
            if let Some(textarea) = self.input_ref.cast::<HtmlTextAreaElement>() {
                // DOM selection offsets are UTF-16 code units, not Unicode scalar values or UTF-8.
                let length = self.input.encode_utf16().count() as u32;
                let _ = textarea.set_selection_range(length, length);
            }
        }
        if mem::take(&mut self.scroll_history_to_end) {
            if let Some(element) = self.history_ref.cast::<web_sys::Element>() {
                element.set_scroll_top(element.scroll_height());
            }
        }
    }
}

fn render_history_entry(entry: &TerminalEntry) -> Html {
    let (icon_class, icon) = match entry.state {
        RunState::Ok => ("stdout-icon", IconData::BOOTSTRAP_CHECK_LG),
        RunState::Err => ("stderr-icon", IconData::FONT_AWESOME_SOLID_XMARK),
    };
    html! {
        <li class="history-entry">
            <div class="history-command">
                <Icon
                    class={icon_class}
                    data={icon}
                    height="10px"
                    width="15px"
                />
                <code class="command">{render_source(&entry.source, &entry.analysis)}</code>
            </div>
            <div class="history-output">{entry.output.clone()}</div>
        </li>
    }
}

fn render_source(source: &str, analysis: &SyntaxAnalysis) -> Html {
    html! {
        {for analysis.tokens().iter().map(|token| {
            let range = token.range();
            let class = syntax_class(token.kind(), token.depth());
            html!(<span class={class}>{&source[range.start()..range.end()]}</span>)
        })}
    }
}

fn syntax_class(kind: SyntaxKind, depth: u16) -> Classes {
    let category = match kind {
        SyntaxKind::Plain => "syntax-plain",
        SyntaxKind::Whitespace => "syntax-whitespace",
        SyntaxKind::Comment => "syntax-comment",
        SyntaxKind::Command => "syntax-command",
        SyntaxKind::Word => "syntax-word",
        SyntaxKind::String => "syntax-string",
        SyntaxKind::Variable => "syntax-variable",
        SyntaxKind::Escape => "syntax-escape",
        SyntaxKind::Delimiter => "syntax-delimiter",
        SyntaxKind::Separator => "syntax-separator",
        SyntaxKind::Number => "syntax-number",
        SyntaxKind::Operator => "syntax-operator",
        SyntaxKind::Function => "syntax-function",
        SyntaxKind::Invalid => "syntax-invalid",
        _ => "syntax-plain",
    };
    if kind == SyntaxKind::Delimiter {
        classes!(category, format!("syntax-depth-{}", depth % 4))
    } else {
        classes!(category)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ping(_interp: &mut Interp<()>, _argv: &[Value]) -> MoltResult {
        molt_ok!("pong")
    }

    #[test]
    fn command_macro_works_through_wasm_reexport() {
        let command = molt::gen_subcommand!((), 1, [("ping", ping, "reply with pong")]);
        let mut interp = Interp::default();
        assert_eq!(
            command(&mut interp, &["bridge".into(), "ping".into()])
                .unwrap()
                .as_str(),
            "pong"
        );
    }

    #[test]
    fn enter_uses_parser_status_and_ime_state() {
        assert_eq!(
            enter_action(syntax::analyze_script("set a {").status(), false, false),
            EnterAction::InsertNewline
        );
        assert_eq!(
            enter_action(syntax::analyze_script("set a {x}tail").status(), false, false),
            EnterAction::Submit
        );
        assert_eq!(
            enter_action(ParseStatus::Complete, true, false),
            EnterAction::InsertNewline
        );
        assert_eq!(enter_action(ParseStatus::Complete, false, true), EnterAction::Ignore);
    }

    #[test]
    fn history_entry_caches_source_analysis() {
        let source = "if {$x > 1} {puts ok}".to_owned();
        let entry = TerminalEntry::new(RunState::Ok, source.clone(), html!());
        assert_eq!(entry.source(), source);
        assert_eq!(entry.analysis.status(), ParseStatus::Complete);
        assert!(entry
            .analysis
            .tokens()
            .iter()
            .any(|token| token.kind() == SyntaxKind::Operator));
    }
}
