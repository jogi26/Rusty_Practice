


use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{read, Event, KeyCode},
    execute, queue,
    style::{Print, Stylize},
    terminal::{
        self, disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

type CtResult<T> = std::io::Result<T>;


/// Ensures terminal always resets correctly
struct TermGuard;

impl TermGuard {
    fn init() -> CtResult<Self> {
        let mut stdout = io::stdout();
        #[cfg(windows)]
        {
            let _ = terminal::enable_ansi_support();
            maximize_console_window();
        }
        execute!(stdout, EnterAlternateScreen, Hide, Clear(ClearType::All))?;
        enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for TermGuard {
    fn drop(&mut self) {
        let mut stdout = io::stdout();
        let _ = disable_raw_mode();
        let _ = execute!(stdout, Show, LeaveAlternateScreen);
    }
}

#[cfg(windows)]
fn maximize_console_window() {
    use windows_sys::Win32::System::Console::GetConsoleWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_MAXIMIZE};

    unsafe {
        let hwnd = GetConsoleWindow();
        if hwnd != 0 {
            let _ = ShowWindow(hwnd, SW_MAXIMIZE);
        }
    }
}

struct McQuestion {
    title: &'static str,
    prompt: &'static str,
    a: &'static str,
    b: &'static str,
    c: &'static str,
    d: &'static str,
    correct: char,
    wrong_msg: &'static str,
}

fn term_size() -> (u16, u16) {
    terminal::size().unwrap_or((80, 24))
}

fn clear(stdout: &mut impl Write) -> CtResult<()> {
    queue!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
    Ok(())
}

fn center_x(width: u16, text: &str) -> u16 {
    let len = text.chars().count() as u16;
    width.saturating_sub(len).saturating_div(2)
}

fn draw_status_bar(
    stdout: &mut impl Write,
    width: u16,
    height: u16,
    lock_idx: usize,
    lock_total: usize,
    status: &str,
    started: Instant,
) -> CtResult<()> {
    let elapsed = Instant::now().duration_since(started);
    let secs = elapsed.as_secs();
    let time = format!("{:02}:{:02}", secs / 60, secs % 60);

    let bar_y = height.saturating_sub(1);
    let fill = " ".repeat(width as usize);

    queue!(
        stdout,
        MoveTo(0, bar_y),
        Print(fill.black().on_dark_grey()),
        MoveTo(1, bar_y),
        Print(format!("LOCK {}/{}", lock_idx, lock_total).black().on_dark_grey().bold()),
        MoveTo(center_x(width, &time), bar_y),
        Print(time.black().on_dark_grey().bold()),
        MoveTo(width.saturating_sub(status.len() as u16 + 9), bar_y),
        Print(format!("STATUS: {}", status).black().on_dark_grey().bold()),
    )?;
    Ok(())
}

fn draw_frame(
    stdout: &mut impl Write,
    title: &str,
    lines: &[String],
    footer: &str,
    lock_idx: usize,
    lock_total: usize,
    status: &str,
    started: Instant,
) -> CtResult<()> {
    let (w, h) = term_size();
    clear(stdout)?;

    queue!(
        stdout,
        MoveTo(center_x(w, title), 1),
        Print(title.bold())
    )?;

    for (i, line) in lines.iter().enumerate() {
        let y = 4 + i as u16;
        queue!(
            stdout,
            MoveTo(center_x(w, line), y),
            Print(line.clone())
        )?;
    }

    queue!(
        stdout,
        MoveTo(1, h.saturating_sub(3)),
        Print(footer.dim())
    )?;

    draw_status_bar(stdout, w, h, lock_idx, lock_total, status, started)?;
    stdout.flush()?;
    Ok(())
}

fn wait_any_key() -> CtResult<()> {
    loop {
        if let Event::Key(_) = read()? {
            break;
        }
    }
    Ok(())
}

fn read_abcd() -> CtResult<char> {
    loop {
        if let Event::Key(k) = read()? {
            if let KeyCode::Char(c) = k.code {
                let c = c.to_ascii_uppercase();
                if matches!(c, 'A' | 'B' | 'C' | 'D') {
                    return Ok(c);
                }
            }
        }
    }
}

fn read_yn() -> CtResult<char> {
    loop {
        if let Event::Key(k) = read()? {
            if let KeyCode::Char(c) = k.code {
                let c = c.to_ascii_uppercase();
                if matches!(c, 'Y' | 'N') {
                    return Ok(c);
                }
            }
        }
    }
}

fn jet_cutscene(stdout: &mut impl Write, started: Instant) -> CtResult<()> {
    let (w, h) = term_size();
    let y = h / 2;

    let jet1 = "    __|__";
    let jet2 = "--o--(_)--o--";

    for x in 0..(w.saturating_sub(jet2.len() as u16)) {
        draw_frame(
            stdout,
            "✈ OPERATION: VALENTINE SORTIE ✈",
            &vec!["Cleared for takeoff…".green().to_string()],
            "Enjoy the flyby 😄",
            0,
            4,
            "RUNNING",
            started,
        )?;
        queue!(
            stdout,
            MoveTo(x, y),
            Print(jet1),
            MoveTo(x, y + 1),
            Print(jet2)
        )?;
        stdout.flush()?;
        thread::sleep(Duration::from_millis(18));
    }
    Ok(())
}

fn ask_mc(
    stdout: &mut impl Write,
    q: &McQuestion,
    idx: usize,
    total: usize,
    started: Instant,
) -> CtResult<()> {
    loop {
        let lines = vec![
            q.prompt.to_string(),
            "".into(),
            format!("A) {}", q.a),
            format!("B) {}", q.b),
            format!("C) {}", q.c),
            format!("D) {}", q.d),
            "".into(),
            "Press A / B / C / D".into(),
        ];

        draw_frame(
            stdout,
            q.title,
            &lines,
            "Choose wisely 🙂",
            idx,
            total,
            "AWAITING INPUT",
            started,
        )?;

        let c = read_abcd()?;
        if c == q.correct {
            draw_frame(
                stdout,
                q.title,
                &vec!["✅ Correct!".green().to_string()],
                "Press any key to continue",
                idx,
                total,
                "PASS",
                started,
            )?;
            wait_any_key()?;
            break;
        } else {
            draw_frame(
                stdout,
                q.title,
                &vec![
                    "❌ Incorrect.".red().to_string(),
                    q.wrong_msg.into(),
                ],
                "Press any key to retry",
                idx,
                total,
                "RETRY",
                started,
            )?;
            wait_any_key()?;
        }
    }
    Ok(())
}

fn final_lock(stdout: &mut impl Write, started: Instant) -> CtResult<()> {
    let mut no_count = 0;

    loop {
        draw_frame(
            stdout,
            "FINAL LOCK",
            &vec![
                "Will you be my Valentine? (Y / N)".into(),
                "(This is easy right? right? 😅)".dim().to_string(),
            ],
            "Press Y or N",
            4,
            4,
            "AWAITING INPUT",
            started,
        )?;

        match read_yn()? {
            'Y' => {
                draw_frame(
                    stdout,
                    "MISSION SUCCESS",
                    &vec![
                        "✈ TAKEOFF CLEARED ✈".green().to_string(),
                        "VALENTINE AUTHORIZED ❤️".into(),
                    ],
                    "Press any key to exit",
                    4,
                    4,
                    "SUCCESS",
                    started,
                )?;
                wait_any_key()?;
                break;
            }
            'N' => {
                no_count += 1;
                let msg = match no_count {
                    1 => "❌ I think you pressed the wrong key...",
                    2 => "❌ Hint: 3 letters.",
                    _ => "❌ Just kidding, I know you love me 😄",
                };
                draw_frame(
                    stdout,
                    "FINAL LOCK",
                    &vec![msg.into()],
                    "Press any key to retry",
                    4,
                    4,
                    "RETRY",
                    started,
                )?;
                wait_any_key()?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn main() -> CtResult<()> {
    let _guard = TermGuard::init()?;
    let mut stdout = io::stdout();
    let started = Instant::now();

    let locks = vec![
        McQuestion {
            title: "LOCK 1: FIRST DATE",
            prompt: "Where was our first date?",
            a: "Kiitsu",
            b: "Raising Canes",
            c: "Six Flags",
            d: "San Diego",
            correct: 'A',
            wrong_msg: "Hint: You like sushi don't you? 🍣",
        },
        McQuestion {
            title: "LOCK 2: FIRST HUG",
            prompt: "When did we first hug?",
            a: "Joshua Tree",
            b: "The beach",
            c: "Dining In",
            d: "All of the above",
            correct: 'A',
            wrong_msg: "Hint: flightline chaos 😄",
        },
        McQuestion {
            title: "LOCK 3: I LOVE YOU SO MUCH THAT I'll...",
            prompt: "What game did we play when we were getting to know each other?",
            a: "It Takes Two",
            b: "Overcooked",
            c: "Fortnite",
            d: "Animal Crossing",
            correct: 'C',
            wrong_msg: "Hint: I carried so hard, its a battle royal game! 🎮",
        },
    ];

    draw_frame(
        &mut stdout,
        "OPERATION: VALENTINE",
        &vec!["Press any key to begin".into()],
        "Controls: A/B/C/D, Y/N",
        0,
        4,
        "STANDBY",
        started,
    )?;
    wait_any_key()?;

    jet_cutscene(&mut stdout, started)?;

    for (i, q) in locks.iter().enumerate() {
        ask_mc(&mut stdout, q, i + 1, 4, started)?;
    }

    final_lock(&mut stdout, started)?;
    Ok(())
}



