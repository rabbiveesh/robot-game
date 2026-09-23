//! Is it working? A plain report over the attempt log, for the parent.
//!
//! Three questions, three sections (see `docs/grouping-axis-spec.md` §Sensing
//! and the measurement notes in `docs/cra-visualization-research.md`):
//!
//! 1. **Is the kid learning?** Unaided work only — no Show me, no Tell me —
//!    per day and per operation, early vs late: first-try accuracy, how long
//!    the first answer takes, how often help is asked for, and CRA stage.
//! 2. **How is the kid solving it?** For unaided, correct additions, does the
//!    answer time grow with the total (counting everything), with the smaller
//!    number (counting on), or barely at all (knows the fact)? A least-squares
//!    fit of time against each; the better fit wins, if it fits at all.
//! 3. **How is the workspace used?** Built or abandoned, how long building
//!    takes, rows and rods vs singles, and dropped counters (motor friction).
//!
//! One kid is a small sample: the report says "not enough yet" rather than
//! guess, and nothing here is a test the kid ever sees.

use std::collections::BTreeMap;
use std::fmt;

use crate::learning::attempt_log::{AttemptRecord, Help};
use crate::types::{CraStage, Operation};

/// Fewer than this and a comparison says "not enough yet".
const MIN_FOR_TREND: usize = 8;
/// Samples needed to call a strategy. With twice this, the estimate splits
/// into the first and second half (like the per-operation trends), so a change
/// of strategy shows up instead of blurring into one muddy fit.
const MIN_FOR_STRATEGY: usize = 12;
/// Answer times outside this window (ms) are dropped from timing stats: a
/// lucky tap, or the kid wandered off mid-question.
const TIMING_WINDOW: (u32, u32) = (400, 60_000);
/// Below this share of variance explained, time "barely depends" on the numbers.
const FLAT_R2: f64 = 0.2;

#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub attempts: usize,
    pub days: Vec<DayLine>,
    pub operations: Vec<OpLine>,
    /// How additions get solved lately (the second half, once there's enough
    /// data to split; everything so far until then)…
    pub strategy: StrategyLine,
    /// …and in the first half, once there's enough to split.
    pub strategy_earlier: Option<StrategyLine>,
    pub workspace: WorkspaceLine,
}

/// One calendar day (UTC) of play.
#[derive(Debug, Clone, PartialEq)]
pub struct DayLine {
    /// "2026-09-24", or "unknown" for records without a clock.
    pub day: String,
    pub attempts: usize,
    pub unaided: usize,
    /// First-try accuracy on unaided attempts.
    pub unaided_accuracy: Option<f64>,
    /// Median first-answer time on unaided, correct attempts.
    pub unaided_median_ms: Option<u32>,
    /// Share of attempts where Show me or Tell me was used.
    pub help_rate: f64,
}

/// One operation, early half vs late half of its attempts.
#[derive(Debug, Clone, PartialEq)]
pub struct OpLine {
    pub operation: Operation,
    pub attempts: usize,
    pub early: Half,
    pub late: Half,
    pub first_stage: CraStage,
    pub last_stage: CraStage,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Half {
    pub attempts: usize,
    pub unaided: usize,
    pub unaided_accuracy: Option<f64>,
    pub unaided_median_ms: Option<u32>,
    pub help_rate: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Strategy {
    /// Time grows with the total: counting everything from one.
    CountingAll,
    /// Time grows with the smaller number: counting on from the bigger one.
    CountingOn,
    /// Time barely depends on the numbers: recalling facts.
    Recall,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fit {
    /// Extra milliseconds per unit of the predictor.
    pub slope_ms: f64,
    pub r2: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StrategyLine {
    /// Unaided, first-try-correct additions with a usable time.
    pub samples: usize,
    pub by_total: Option<Fit>,
    pub by_smaller: Option<Fit>,
    /// `None` until there are enough samples.
    pub verdict: Option<Strategy>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct WorkspaceLine {
    pub opened: usize,
    pub built: usize,
    /// Answered before finishing the model (skipped ahead, or gave up on it).
    pub answered_before_built: usize,
    pub median_build_ms: Option<u32>,
    pub singles: u32,
    pub singles_past_a_row: u32,
    pub rows: u32,
    pub rods_snapped: u32,
    pub rods_opened: u32,
    pub misses: u32,
    /// First-try accuracy on the attempts where the workspace was opened.
    pub first_try_accuracy: Option<f64>,
}

/// Build the report. `records` in any order; they're sorted by time here.
pub fn analyze(records: &[AttemptRecord]) -> Report {
    let mut rs: Vec<&AttemptRecord> = records.iter().collect();
    rs.sort_by(|x, y| x.at.total_cmp(&y.at).then(x.play_secs.total_cmp(&y.play_secs)));
    Report {
        attempts: rs.len(),
        days: days(&rs),
        operations: operations(&rs),
        strategy: strategy_recent(&rs),
        strategy_earlier: strategy_earlier(&rs),
        workspace: workspace(&rs),
    }
}

fn median(mut xs: Vec<u32>) -> Option<u32> {
    if xs.is_empty() {
        return None;
    }
    xs.sort_unstable();
    Some(xs[xs.len() / 2])
}

fn usable_ms(r: &AttemptRecord) -> Option<u32> {
    r.first_answer_ms().filter(|ms| (TIMING_WINDOW.0..=TIMING_WINDOW.1).contains(ms))
}

fn half(rs: &[&AttemptRecord]) -> Half {
    let unaided: Vec<_> = rs.iter().filter(|r| r.unaided()).collect();
    let right = unaided.iter().filter(|r| r.first_try_correct()).count();
    Half {
        attempts: rs.len(),
        unaided: unaided.len(),
        unaided_accuracy: (!unaided.is_empty()).then(|| right as f64 / unaided.len() as f64),
        unaided_median_ms: median(
            unaided.iter().filter(|r| r.first_try_correct()).filter_map(|r| usable_ms(r)).collect(),
        ),
        help_rate: if rs.is_empty() { 0.0 } else { rs.iter().filter(|r| !r.unaided()).count() as f64 / rs.len() as f64 },
    }
}

fn days(rs: &[&AttemptRecord]) -> Vec<DayLine> {
    let mut by_day: BTreeMap<i64, Vec<&AttemptRecord>> = BTreeMap::new();
    for r in rs {
        let day = if r.at > 0.0 { (r.at / 86_400.0).floor() as i64 } else { i64::MIN };
        by_day.entry(day).or_default().push(r);
    }
    by_day
        .into_iter()
        .map(|(day, rs)| {
            let h = half(&rs);
            DayLine {
                day: if day == i64::MIN { "unknown".into() } else { civil_date(day) },
                attempts: h.attempts,
                unaided: h.unaided,
                unaided_accuracy: h.unaided_accuracy,
                unaided_median_ms: h.unaided_median_ms,
                help_rate: h.help_rate,
            }
        })
        .collect()
}

fn operations(rs: &[&AttemptRecord]) -> Vec<OpLine> {
    let mut by_op: Vec<(Operation, Vec<&AttemptRecord>)> = Vec::new();
    for r in rs {
        match by_op.iter_mut().find(|(op, _)| *op == r.operation) {
            Some((_, v)) => v.push(r),
            None => by_op.push((r.operation, vec![r])),
        }
    }
    by_op
        .into_iter()
        .map(|(operation, v)| {
            let mid = v.len() / 2;
            OpLine {
                operation,
                attempts: v.len(),
                early: half(&v[..mid]),
                late: half(&v[mid..]),
                first_stage: v[0].cra_stage,
                last_stage: v[v.len() - 1].cra_stage,
            }
        })
        .collect()
}

/// Least-squares fit of `y` on `x`: slope and share of variance explained.
fn fit(points: &[(f64, f64)]) -> Option<Fit> {
    let n = points.len() as f64;
    if points.len() < 3 {
        return None;
    }
    let (mx, my) = (points.iter().map(|p| p.0).sum::<f64>() / n, points.iter().map(|p| p.1).sum::<f64>() / n);
    let sxx: f64 = points.iter().map(|p| (p.0 - mx).powi(2)).sum();
    let syy: f64 = points.iter().map(|p| (p.1 - my).powi(2)).sum();
    let sxy: f64 = points.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
    if sxx == 0.0 || syy == 0.0 {
        return None;
    }
    Some(Fit { slope_ms: sxy / sxx, r2: (sxy * sxy) / (sxx * syy) })
}

/// (total, smaller addend, ms) for unaided, right-first-time additions, in
/// time order.
fn strategy_samples(rs: &[&AttemptRecord]) -> Vec<(f64, f64, f64)> {
    rs.iter()
        .filter(|r| r.operation == Operation::Add && r.format == "standard" && r.unaided() && r.first_try_correct())
        .filter_map(|r| usable_ms(r).map(|ms| ((r.a + r.b) as f64, r.a.min(r.b) as f64, ms as f64)))
        .collect()
}

/// Where the "earlier" half ends, once there's enough to split.
fn strategy_split(n: usize) -> usize {
    if n >= 2 * MIN_FOR_STRATEGY { n / 2 } else { 0 }
}

fn strategy_recent(rs: &[&AttemptRecord]) -> StrategyLine {
    let all = strategy_samples(rs);
    strategy(&all[strategy_split(all.len())..])
}

fn strategy_earlier(rs: &[&AttemptRecord]) -> Option<StrategyLine> {
    let all = strategy_samples(rs);
    let split = strategy_split(all.len());
    (split > 0).then(|| strategy(&all[..split]))
}

fn strategy(samples: &[(f64, f64, f64)]) -> StrategyLine {
    let by_total = fit(&samples.iter().map(|s| (s.0, s.2)).collect::<Vec<_>>());
    let by_smaller = fit(&samples.iter().map(|s| (s.1, s.2)).collect::<Vec<_>>());
    let verdict = (samples.len() >= MIN_FOR_STRATEGY).then(|| {
        let r2 = |f: &Option<Fit>| f.as_ref().map_or(0.0, |f| f.r2);
        let (t, s) = (r2(&by_total), r2(&by_smaller));
        if t.max(s) < FLAT_R2 {
            Strategy::Recall
        } else if s > t {
            Strategy::CountingOn
        } else {
            Strategy::CountingAll
        }
    });
    StrategyLine { samples: samples.len(), by_total, by_smaller, verdict }
}

fn workspace(rs: &[&AttemptRecord]) -> WorkspaceLine {
    let used: Vec<_> = rs.iter().filter(|r| r.help == Help::Workspace).collect();
    let mut w = WorkspaceLine { opened: used.len(), ..Default::default() };
    let mut build_times = Vec::new();
    for r in &used {
        let Some(u) = &r.workspace else { continue };
        if let Some(ms) = u.built_ms {
            w.built += 1;
            build_times.push(ms);
        }
        // Answered while the model was unfinished: before it was built, or
        // it never was.
        let built_at = u.built_ms.zip(r.help_ms).map(|(b, h)| b + h);
        let first = r.first_answer_ms();
        if first.is_some() && built_at.map_or(true, |b| first.is_some_and(|f| f < b)) {
            w.answered_before_built += 1;
        }
        w.singles += u.singles as u32;
        w.singles_past_a_row += u.singles_past_a_row as u32;
        w.rows += u.rows as u32;
        w.rods_snapped += u.rods_snapped as u32;
        w.rods_opened += u.rods_opened as u32;
        w.misses += u.misses as u32;
    }
    w.median_build_ms = median(build_times);
    if !used.is_empty() {
        w.first_try_accuracy = Some(used.iter().filter(|r| r.first_try_correct()).count() as f64 / used.len() as f64);
    }
    w
}

/// Days since the Unix epoch → "YYYY-MM-DD" (proleptic Gregorian, UTC).
fn civil_date(days: i64) -> String {
    // Howard Hinnant's days_from_civil, inverted.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}")
}

// ─── TEXT ───────────────────────────────────────────────

fn pct(x: Option<f64>) -> String {
    x.map_or("  —".into(), |x| format!("{:>3.0}%", x * 100.0))
}

fn secs(ms: Option<u32>) -> String {
    ms.map_or("   —".into(), |ms| format!("{:>4.1}s", ms as f64 / 1000.0))
}

fn op_name(op: Operation) -> &'static str {
    match op {
        Operation::Add => "addition",
        Operation::Sub => "subtraction",
        Operation::Multiply => "multiplication",
        Operation::Divide => "division",
        Operation::NumberBond => "number bonds",
    }
}

fn write_strategy(f: &mut fmt::Formatter<'_>, when: &str, s: &StrategyLine) -> fmt::Result {
    writeln!(f, "  {when} ({} answers)", s.samples)?;
    let line = |name: &str, fit: &Option<Fit>| match fit {
        Some(fit) => format!("    time vs {name}: +{:.0}ms per step, fits {:.0}%", fit.slope_ms, fit.r2 * 100.0),
        None => format!("    time vs {name}: —"),
    };
    writeln!(f, "{}", line("the total", &s.by_total))?;
    writeln!(f, "{}", line("the smaller number", &s.by_smaller))?;
    writeln!(
        f,
        "    → {}",
        match s.verdict {
            None => "not enough yet",
            Some(Strategy::CountingAll) => "counting everything (time grows with the total)",
            Some(Strategy::CountingOn) => "counting on (time grows with the smaller number)",
            Some(Strategy::Recall) => "recalling facts (time barely depends on the numbers)",
        }
    )
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} challenges", self.attempts)?;
        if self.attempts == 0 {
            return Ok(());
        }

        writeln!(f, "\nBy day (unaided = no Show me, no Tell me)")?;
        writeln!(f, "  day          tries  unaided  first-try  time   help")?;
        for d in &self.days {
            writeln!(
                f,
                "  {:<12} {:>5}  {:>7}  {:>9}  {}  {:>4.0}%",
                d.day, d.attempts, d.unaided, pct(d.unaided_accuracy), secs(d.unaided_median_ms), d.help_rate * 100.0
            )?;
        }

        writeln!(f, "\nBy operation, first half of tries → second half")?;
        for o in &self.operations {
            writeln!(f, "  {} ({} tries; stage {:?} → {:?})", op_name(o.operation), o.attempts, o.first_stage, o.last_stage)?;
            if o.attempts < MIN_FOR_TREND {
                writeln!(f, "    not enough yet")?;
                continue;
            }
            writeln!(
                f,
                "    unaided first-try {} → {}   time {} → {}   help {:.0}% → {:.0}%",
                pct(o.early.unaided_accuracy), pct(o.late.unaided_accuracy),
                secs(o.early.unaided_median_ms), secs(o.late.unaided_median_ms),
                o.early.help_rate * 100.0, o.late.help_rate * 100.0,
            )?;
        }

        writeln!(f, "\nHow additions get solved (unaided, right-first-time)")?;
        if let Some(earlier) = &self.strategy_earlier {
            write_strategy(f, "earlier", earlier)?;
        }
        write_strategy(f, "lately", &self.strategy)?;

        let w = &self.workspace;
        writeln!(f, "\nHands-on workspace")?;
        if w.opened == 0 {
            return writeln!(f, "  not used yet");
        }
        writeln!(
            f,
            "  opened {}   built {}   answered before finishing {}   median build {}",
            w.opened, w.built, w.answered_before_built, secs(w.median_build_ms)
        )?;
        writeln!(
            f,
            "  moves: {} singles ({} with a whole row there), {} rows of five   rods: {} snapped, {} opened   dropped off target: {}",
            w.singles, w.singles_past_a_row, w.rows, w.rods_snapped, w.rods_opened, w.misses
        )?;
        writeln!(f, "  right first time after using it: {}", pct(w.first_try_accuracy))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::learning::attempt_log::tests::attempt;
    use crate::learning::attempt_log::{WorkspaceKind, WorkspaceUse};

    fn additions(ms: impl Fn(i32, i32) -> u32) -> Vec<AttemptRecord> {
        let mut out = Vec::new();
        for a in 1..=6 {
            for b in 1..=4 {
                let mut r = attempt(a, b, a + b, ms(a, b));
                r.at += (out.len() * 60) as f64;
                out.push(r);
            }
        }
        out
    }

    #[test]
    fn time_growing_with_the_total_reads_as_counting_everything() {
        let rep = analyze(&additions(|a, b| 1000 + 600 * (a + b) as u32));
        assert_eq!(rep.strategy.verdict, Some(Strategy::CountingAll));
    }

    #[test]
    fn time_growing_with_the_smaller_number_reads_as_counting_on() {
        let rep = analyze(&additions(|a, b| 1500 + 900 * a.min(b) as u32));
        assert_eq!(rep.strategy.verdict, Some(Strategy::CountingOn));
    }

    #[test]
    fn flat_times_read_as_recall() {
        // Small wobble unrelated to the numbers.
        let rep = analyze(&additions(|a, b| 1200 + ((a * 7 + b * 13) % 5) as u32 * 40));
        assert_eq!(rep.strategy.verdict, Some(Strategy::Recall));
    }

    #[test]
    fn a_change_of_strategy_shows_as_earlier_then_lately() {
        let mut rs = additions(|a, b| 1000 + 600 * (a + b) as u32); // 24: counting everything
        let later = additions(|a, b| 1500 + 900 * a.min(b) as u32); // 24: counting on
        let offset = rs.len() as f64 * 60.0;
        rs.extend(later.into_iter().map(|mut r| {
            r.at += offset;
            r
        }));
        rep_has(&analyze(&rs), Some(Strategy::CountingAll), Some(Strategy::CountingOn));
    }

    fn rep_has(rep: &Report, earlier: Option<Strategy>, lately: Option<Strategy>) {
        // 48 samples split 24 / 24.
        assert_eq!(rep.strategy_earlier.as_ref().and_then(|s| s.verdict), earlier);
        assert_eq!(rep.strategy.verdict, lately);
    }

    #[test]
    fn a_handful_of_samples_is_not_enough_to_call_it() {
        let few: Vec<_> = additions(|a, b| 1000 + 600 * (a + b) as u32).into_iter().take(5).collect();
        assert_eq!(analyze(&few).strategy.verdict, None);
    }

    #[test]
    fn helped_attempts_stay_out_of_the_unaided_numbers() {
        let mut rs = additions(|_, _| 2000);
        for r in rs.iter_mut().take(10) {
            r.help = Help::Workspace;
            r.answers[0].value += 1; // wrong, but with help
        }
        let rep = analyze(&rs);
        let add = &rep.operations[0];
        assert_eq!(add.early.unaided_accuracy.unwrap(), 1.0, "the wrong ones were all helped");
        assert!(add.early.help_rate > add.late.help_rate);
    }

    #[test]
    fn days_group_by_calendar_date() {
        let mut rs = additions(|_, _| 2000);
        rs[0].at = 0.0;
        let rep = analyze(&rs);
        assert_eq!(rep.days[0].day, "unknown");
        assert_eq!(rep.days[1].day, "2026-09-21");
    }

    #[test]
    fn workspace_use_is_totalled() {
        let mut r = attempt(8, 5, 13, 9000);
        r.help = Help::Workspace;
        r.help_ms = Some(1000);
        r.workspace = Some(WorkspaceUse {
            kind: Some(WorkspaceKind::PutTogether),
            built_ms: Some(6000),
            singles: 8,
            singles_past_a_row: 3,
            rows: 1,
            misses: 2,
            ..Default::default()
        });
        let mut early = r.clone();
        early.answers[0].ms = 3000; // answered before the model was done
        let rep = analyze(&[r, early]);
        let w = &rep.workspace;
        assert_eq!((w.opened, w.built, w.answered_before_built), (2, 2, 1));
        assert_eq!((w.singles, w.rows, w.misses), (16, 2, 4));
    }

    #[test]
    fn the_report_prints() {
        let text = analyze(&additions(|a, b| 1000 + 600 * (a + b) as u32)).to_string();
        assert!(text.contains("counting everything"));
        assert!(text.contains("addition"));
    }
}
