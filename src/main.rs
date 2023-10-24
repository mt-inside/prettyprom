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

fn main() -> anyhow::Result<()> {
    // TODO: tests!

    let mut lines = std::io::stdin().lines();
    let mut hack_help_line = None;

    'all: loop {
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

        'metrics: loop {
            // On EOF / read error; ie no more input, stop
            if let Some(ref metric) = lines.next().and_then(|l| l.ok()) {
                // On parse error, assume we're at the end of the metrics block and go back to beginning
                if let Ok((metric_name, labels, val)) = parse_metric(metric) {
                    assert!(metric_name.starts_with(name));
                    // TODO: handle hists etc. Will get a foo_sum (and count?); at least handle that. Ideally render hist buckets (by other labels). Needs the parse_metricS()

                    print!(
                        "  {}\t",
                        format!("{}", val)
                            .with(Color::White)
                            .attribute(Attribute::Bold)
                    );
                    for (k, v) in labels {
                        print!("{}: {} ", k.with(Color::Blue), v.with(Color::Green));
                    }
                    println!();
                } else {
                    hack_help_line = Some(metric.clone());
                    break 'metrics;
                }
            } else {
                break 'all;
            }
        }
    }

    Ok(())
}
