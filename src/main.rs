use crossterm::style::{Attribute, Color, Stylize};
use nom::{
    bytes::complete::{tag, take_till, take_while1},
    character::complete::{alpha1 as metrictype, alphanumeric1 as labelkey, char, not_line_ending},
    character::is_alphanumeric,
    combinator::opt,
    multi::separated_list1,
    number::complete::float,
    sequence::{delimited, pair, separated_pair, tuple},
};

fn is_metricname(c: char) -> bool {
    is_alphanumeric(c as u8) || c == '_'
}

fn parse_help(line: &str) -> (&str, &str) {
    let mut p_help = tuple((
        tag::<&str, &str, nom::error::Error<&str>>("# HELP"),
        char(' '),
        take_while1(is_metricname),
        char(' '),
        not_line_ending,
    ));

    let (remain, (_, _, name, _, desc)) = p_help(line).unwrap();
    assert_eq!(remain, "");
    (name, desc)
}
fn parse_type(line: &str) -> (&str, &str) {
    let mut p_type = tuple((
        tag::<&str, &str, nom::error::Error<&str>>("# TYPE"),
        char(' '),
        take_while1(is_metricname),
        char(' '),
        metrictype,
    ));

    let (remain, (_, _, name, _, typ)) = p_type(line).unwrap();
    assert_eq!(remain, "");
    (name, typ)
}
fn parse_metric(
    line: &str,
) -> Result<(&str, Vec<(&str, &str)>, f32), nom::Err<nom::error::Error<&str>>> {
    let mut p_metric = separated_pair(
        pair(
            take_while1(is_metricname),
            opt(delimited(
                char('{'),
                separated_list1(
                    char(','),
                    separated_pair(
                        labelkey,
                        char('='),
                        delimited(char('\"'), take_till(|c| c == '\"'), char('\"')),
                    ),
                ),
                char('}'),
            )),
        ),
        char(' '),
        float,
    );

    let (remain, ((name, labels), val)) = p_metric(line)?;
    assert_eq!(remain, "");
    Ok((name, labels.unwrap_or(vec![]), val))
}

trait MetricParser {
    fn parse_line(&mut self, line_name: &str, labels: Vec<(&str, &str)>, val: f32);
    fn render(&self);
}

struct GaugeCounterParser {}

impl GaugeCounterParser {
    fn new() -> Self {
        Self {}
    }
}

impl MetricParser for GaugeCounterParser {
    fn parse_line(&mut self, line_name: &str, labels: Vec<(&str, &str)>, val: f32) {
        print!(
            "  {}\t",
            format!("{}", val)
                .with(Color::White)
                .attribute(Attribute::Bold)
        );
        for (k, v) in labels {
            print!("{}={} ", k.with(Color::Blue), v.with(Color::Green));
        }
        println!();
    }
    fn render(&self) {
        println!("TODO");
    }
}

struct HistSummaryParser {
    metric_name: String, // TODO: can borrow?
    bs: Vec<(String, f32)>,
}

impl HistSummaryParser {
    fn new(metric_name: &str) -> Self {
        Self {
            metric_name: metric_name.to_owned(),
            bs: vec![],
        }
    }
}

impl MetricParser for HistSummaryParser {
    fn parse_line(&mut self, line_name: &str, labels: Vec<(&str, &str)>, val: f32) {
        // TODO: factor out with hist
        // TODO: calc mean for both types
        let suffix = if line_name == self.metric_name {
            ""
        } else {
            &line_name[self.metric_name.len() + 1..]
        };
        match suffix {
            "" => {
                // Summary
                for (k, v) in labels {
                    if k == "quantile" {
                        self.bs.push((v.to_owned(), val));
                    }
                }
            }
            "bucket" => {
                // Histogram
                for (k, v) in labels {
                    if k == "le" {
                        self.bs.push((v.to_owned(), val));
                    }
                }
            }
            "sum" => {
                print!("  ");
                print!(
                    "{} {} ",
                    suffix,
                    format!("{}", val)
                        .with(Color::White)
                        .attribute(Attribute::Bold)
                );
            }
            "count" => {
                print!(
                    "{} {} ",
                    suffix,
                    format!("{}", val)
                        .with(Color::White)
                        .attribute(Attribute::Bold)
                );
                print!("(");
                for (k, v) in &self.bs {
                    print!("{} {}, ", k.clone().with(Color::DarkGrey), v);
                }
                print!(")");
                self.bs.clear(); // TODO unnecessary now?
                print!(" ");
                for (k, v) in labels {
                    if k != "le" {
                        print!("{}={} ", k.with(Color::Blue), v.with(Color::Green));
                    }
                }
                println!()
            }
            _ => panic!("Unknown suffix {}", suffix),
        }
    }
    fn render(&self) {
        println!("TODO");
    }
}

fn parse_metrics(
    lines: &mut std::io::Lines<std::io::StdinLock<'static>>,
    metric_name: &str,
    parser: &mut Box<dyn MetricParser>,
) -> ((), Option<String>) {
    loop {
        // On EOF / read error; ie no more input, stop
        if let Some(ref metric) = lines.next().and_then(|l| l.ok()) {
            // On parse error, assume we're at the end of the metrics block and go back to beginning
            if let Ok((line_name, labels, val)) = parse_metric(metric) {
                assert!(line_name.starts_with(metric_name));

                parser.parse_line(line_name, labels, val);
            } else {
                return ((), Some(metric.to_owned()));
            }
        } else {
            return ((), None);
        }
    }
}

fn main() -> anyhow::Result<()> {
    // TODO: tests!

    let mut lines = std::io::stdin().lines();
    let mut hack_help_line = None;

    loop {
        let help_line = hack_help_line.unwrap_or_else(|| lines.next().unwrap().unwrap());
        let type_line = lines.next().unwrap().unwrap();

        let (name, desc) = parse_help(&help_line);
        let (type_name, typ) = parse_type(&type_line);
        assert_eq!(type_name, name);

        // TODO: show total for this metric. Also sort the outputs by value - have a
        // parse_metricS() which returns Vec<Vec<(String, String)>, i64>. Should make the control
        // flow better
        println!(
            "{} {} \"{}\"",
            name.attribute(Attribute::Bold),
            typ.with(Color::DarkGrey),
            desc,
        );

        let mut parser: Box<dyn MetricParser> = match typ {
            "gauge" | "counter" => Box::new(GaugeCounterParser::new()),
            "histogram" | "summary" => Box::new(HistSummaryParser::new(name)),
            _ => panic!("Unknown metric type"),
        };
        (_, hack_help_line) = parse_metrics(&mut lines, name, &mut parser);
        if hack_help_line.is_none() {
            break;
        }
    }

    Ok(())
}
