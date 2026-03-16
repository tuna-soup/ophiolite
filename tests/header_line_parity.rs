use lithos_las::parse_header_line;

#[test]
fn parses_time_string_and_colon_in_description() {
    let line = "TIML.hh:mm 23:15 23-JAN-2001:   Time Logger: At Bottom";
    let result = parse_header_line(line, Some("Parameter")).unwrap();
    assert_eq!(result.value, "23:15 23-JAN-2001");
    assert_eq!(result.description, "Time Logger: At Bottom");
}

#[test]
fn parses_datetime_value_with_colons() {
    let line = "STRT.DateTime 2012-09-16T07:44:12-05:00 : START DEPTH";
    let result = parse_header_line(line, Some("Well")).unwrap();
    assert_eq!(result.value, "2012-09-16T07:44:12-05:00");
    assert_eq!(result.description, "START DEPTH");
    assert_eq!(result.unit, "DateTime");
}

#[test]
fn parses_unit_starting_with_dot() {
    let line = " TDEP  ..1IN                      :  0.1-in";
    let result = parse_header_line(line, Some("Curves")).unwrap();
    assert_eq!(result.unit, ".1IN");
}

#[test]
fn parses_name_containing_dot() {
    let line = "I. Res..OHM-M                  ";
    let result = parse_header_line(line, Some("Curves")).unwrap();
    assert_eq!(result.name, "I. Res.");
    assert_eq!(result.unit, "OHM-M");
}

#[test]
fn parses_line_without_period() {
    let line = "              DRILLED  :12/11/2010";
    let result = parse_header_line(line, None).unwrap();
    assert_eq!(result.name, "DRILLED");
    assert_eq!(result.value, "12/11/2010");
}

#[test]
fn parses_value_field_with_numeric_colon() {
    let line = "RUN . 01: RUN NUMBER";
    let result = parse_header_line(line, Some("Parameter")).unwrap();
    assert_eq!(result.value, "01");
    assert_eq!(result.description, "RUN NUMBER");
}

#[test]
fn parses_non_delimiter_colon_in_description() {
    let line = "QI     .      :         Survey quality: GOOD or BAD versus criteria";
    let result = parse_header_line(line, Some("Parameter")).unwrap();
    assert_eq!(result.value, "");
    assert_eq!(
        result.description,
        "Survey quality: GOOD or BAD versus criteria"
    );
}

#[test]
fn parses_unit_with_space() {
    let line = "HKLA            .1000 lbf                                  :(RT)";
    let result = parse_header_line(line, Some("Parameter")).unwrap();
    assert_eq!(result.name, "HKLA");
    assert_eq!(result.unit, "1000 lbf");
    assert_eq!(result.value, "");
    assert_eq!(result.description, "(RT)");
}

#[test]
fn parses_line_without_period_with_time_value() {
    let line = "              TIME     :14:00:32";
    let result = parse_header_line(line, None).unwrap();
    assert_eq!(result.name, "TIME");
    assert_eq!(result.value, "14:00:32");
}
