use crossterm::style::{Attribute, Color, Stylize};
use nom::{
    bytes::complete::{tag, take_till},
    character::complete::{alpha1, alphanumeric1, char, i64, not_line_ending},
    multi::separated_list1,
    sequence::{delimited, pair, separated_pair, tuple},
};

fn parse_help(line: &str) -> (&str, &str) {
    let mut p_help = tuple((
        tag::<&str, &str, nom::error::Error<&str>>("# HELP"),
        char(' '),
        alphanumeric1,
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
        alphanumeric1,
        char(' '),
        alpha1,
    ));

    let (remain, (_, _, name, _, typ)) = p_type(line).unwrap();
    assert_eq!(remain, "");
    (name, typ)
}
fn parse_metric(
    line: &str,
) -> Result<(&str, Vec<(&str, &str)>, i64), nom::Err<nom::error::Error<&str>>> {
    let mut p_metric = separated_pair(
        pair(
            alphanumeric1::<&str, nom::error::Error<&str>>,
            delimited(
                char('{'),
                separated_list1(
                    char(','),
                    separated_pair(
                        alphanumeric1, // TODO: factor these out as metric_name, dimension_label, etc, and confirm format
                        char('='),
                        delimited(char('\"'), take_till(|c| c == '\"'), char('\"')),
                    ),
                ),
                char('}'),
            ),
        ),
        char(' '),
        i64, // TODO: what about floats? What's the rules for the various types? Is it type-speicif? Will nom parse "3" as a float?
    );

    let (remain, ((name, labels), val)) = p_metric(line)?;
    assert_eq!(remain, "");
    Ok((name, labels, val))
}

fn main() -> anyhow::Result<()> {
    // TODO: tests!

    // TODO: check the output of all the different metrics types (from the Golang lib), think about how to render them, esp histograms

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
            // Stop on EOF, read error
            if let Some(ref metric) = lines.next().and_then(|l| l.ok()) {
                if let Ok((metric_name, labels, val)) = parse_metric(metric) {
                    assert_eq!(metric_name, name);

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
