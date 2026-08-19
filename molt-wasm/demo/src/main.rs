use molt_wasm::{molt::prelude::*, RunState, Terminal};
use std::{mem, rc::Rc};
use yew::prelude::*;
use yew_icons::{Icon, IconId};
include!(concat!(env!("OUT_DIR"), "/compile_info.rs"));

const INIT_CMDS: [&str; 9] = [
    "about",
    "proc say_hello {name} {
    puts \"Hello, $name!\"
}",
    "say_hello \"World\"",
    "set a {}",
    "for {set i 1} {$i < 6} {incr i} {
    puts $i
    square $i
    if {$i == 4} {
        break
    }
    lappend a $i
}",
    "set a",
    "square \"it-should-error\"",
    "help -all",
    "browser -help",
];

impl App {
    #[inline]
    fn execute(&mut self, cmd: String) {
        let out = self.interp.eval(&cmd);
        let mut outs = mem::take(&mut self.interp.std_buff);
        outs.push(out);
        Rc::make_mut(&mut self.interp.context.hist)
            .push(Terminal::to_hist(cmd.trim().into(), outs));
    }
}
pub enum AppMsg {
    RunCmd(String, bool),
    ToggleDark,
}

pub fn cmd_square(interp: &mut Interp<AppCtx>, argv: &[Value]) -> MoltResult {
    // Correct number of arguments?
    check_args(1, argv, 2, 2, "x")?;
    // Get x, if it's an integer
    let x = argv[1].as_int()?;
    let out = x * x;
    interp.context.num = out as usize;
    molt_ok!(out)
}

pub fn cmd_about(interp: &mut Interp<AppCtx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 1, 1, "")?;
    molt_ok!(
        "{} {} ({})\n{} {}\nType \"help\" for more information.",
        interp.name,
        CRATE_VERSION,
        COMPILE_TIME,
        RUSTC_VERSION,
        GCC_VERSION
    )
}

pub fn cmd_clear(interp: &mut Interp<AppCtx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 1, 1, "")?;
    Rc::make_mut(&mut interp.context.hist).clear();
    molt_ok!()
}

pub fn cmd_browser_alert(_interp: &mut Interp<AppCtx>, argv: &[Value]) -> MoltResult {
    if let Some(window) = web_sys::window() {
        let input = if let Some(v) = argv.get(2) {
            window.alert_with_message(v.as_str())
        } else {
            window.alert()
        };
        match input {
            Ok(_) => molt_ok!(),
            Err(e) => {
                if let Some(s) = e.as_string() {
                    molt_err!(s)
                } else {
                    molt_err!("Unknown error")
                }
            }
        }
    } else {
        molt_err!("no global `window` exists")
    }
}

pub fn cmd_browser_confirm(_interp: &mut Interp<AppCtx>, argv: &[Value]) -> MoltResult {
    if let Some(window) = web_sys::window() {
        let input = if let Some(v) = argv.get(2) {
            window.confirm_with_message(v.as_str())
        } else {
            window.confirm()
        };
        match input {
            Ok(_) => molt_ok!(),
            Err(e) => {
                if let Some(s) = e.as_string() {
                    molt_err!(s)
                } else {
                    molt_err!("Unknown error")
                }
            }
        }
    } else {
        molt_err!("no global `window` exists")
    }
}

pub fn cmd_browser_prompt(_interp: &mut Interp<AppCtx>, argv: &[Value]) -> MoltResult {
    if let Some(window) = web_sys::window() {
        let input = if let Some(v) = argv.get(2) {
            window.prompt_with_message(v.as_str())
        } else {
            window.prompt()
        };
        match input {
            Ok(Some(s)) => molt_ok!(s),
            Ok(None) => molt_ok!(),
            Err(e) => {
                if let Some(s) = e.as_string() {
                    molt_err!(s)
                } else {
                    molt_err!("Unknown error")
                }
            }
        }
    } else {
        molt_err!("no global `window` exists")
    }
}

#[allow(non_upper_case_globals)]
const cmd_browser: fn(&mut Interp<AppCtx>, &[Value]) -> Result<Value, Exception> = gen_subcommand!(
    AppCtx,
    1,
    [
        ("-alert", cmd_browser_alert, "alert (with message if provided)"),
        ("-confirm", cmd_browser_confirm, "confirm (with message if provided)"),
        ("-prompt", cmd_browser_prompt, "prompt (with message if provided)"),
    ],
);

pub struct AppCtx {
    num: usize,
    pub hist: Rc<Vec<(RunState, String, Html)>>,
}
pub struct App {
    darkmode: bool,
    interp: Interp<AppCtx>,
}

impl Component for App {
    type Message = AppMsg;

    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        let interp = Interp::new(
            AppCtx { num: 0, hist: Rc::new(Vec::new()) },
            gen_command!(
                AppCtx,
                // native commands
                [],
                // embedded commands
                [
                    ("about", cmd_about, "display app information"),
                    ("square", cmd_square, "square input and set app context number"),
                    ("clear", cmd_clear, "clear history"),
                    ("browser", cmd_browser, "call browser APIs"),
                ]
            ),
            false,
            "molt-wasm-demo",
        );
        let mut app = Self { darkmode: true, interp };
        for cmd in INIT_CMDS {
            app.execute(cmd.into());
        }
        app
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            AppMsg::RunCmd(cmd, previous_is_uncompleted) => {
                if previous_is_uncompleted {
                    let previous = Rc::make_mut(&mut self.interp.context.hist).pop();
                    if let Some((_, previous_cmd, previous_out)) = previous {
                        if cmd.trim().is_empty() {
                            // If uncompleted continue with nothing, then just return error
                            Rc::make_mut(&mut self.interp.context.hist).push((
                                RunState::Err,
                                previous_cmd,
                                previous_out,
                            ));
                        } else {
                            self.execute(previous_cmd + "\n" + &cmd)
                        }
                    } else {
                        self.execute(cmd)
                    }
                } else {
                    self.execute(cmd)
                }
            }
            AppMsg::ToggleDark => self.darkmode = !self.darkmode,
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
                <div>
                    <div onclick={ctx.link().callback(|_|AppMsg::ToggleDark)}>
                        <Icon icon_id={if self.darkmode{IconId::FeatherMoon}else{IconId::FeatherSun}} height={"20px".to_owned()} width={"20px".to_owned()}/>
                    </div>
                    <a href="https://github.com/zao111222333/molt-forked/tree/master/molt-wasm/demo"><code>{"code"}</code><Icon icon_id={IconId::BootstrapGithub} height={"10px".to_owned()} width={"15px".to_owned()}/></a>
                    <code>{" The context number is "}</code><code style="color:red;">{self.interp.context.num}</code><code>{", run `square [number]` to change it"}</code>
                </div>
                <Terminal
                    class={if self.darkmode{ "terminal dark" }else{ "terminal" }}
                    hist={self.interp.context.hist.clone()}
                    on_run_cmd={ctx.link().callback(|(cmd,previous_is_uncompleted)|AppMsg::RunCmd(cmd,previous_is_uncompleted))}
                />
            </>
        }
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));
    yew::Renderer::<App>::new().render();
}
