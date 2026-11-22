#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]

    use crate::defines::*;
    use std::collections::HashSet;

    /// Checks that a list has no empty or untrimmed label and no duplicate,
    /// ignoring case.
    fn assert_clean_labels(name: &str, labels: &[&str]) {
        let mut seen = HashSet::new();
        for label in labels {
            assert!(!label.is_empty(), "{name}: empty label");
            assert_eq!(label.trim(), *label, "{name}: untrimmed {label:?}");
            assert!(
                seen.insert(label.to_lowercase()),
                "{name}: duplicate {label:?}"
            );
        }
    }

    #[test]
    fn test_labels_are_clean() {
        assert_clean_labels("TAGS", &TAGS);
        assert_clean_labels("CATEGORIES", &CATEGORIES);
        assert_clean_labels("SUPPLIERS", &SUPPLIERS);
        assert_clean_labels("PRODUCERS", &PRODUCERS);
        assert_clean_labels("SIGNAL_WORDS", &SIGNAL_WORDS);
        assert_clean_labels("PHYSICAL_STATES", &PHYSICAL_STATES);
        assert_clean_labels("SYMBOLS", &SYMBOLS);
        assert_clean_labels("CLASSES_OF_COMPOUNDS", &CLASSES_OF_COMPOUNDS);
    }

    #[test]
    fn test_symbols_are_ghs_pictograms() {
        for symbol in SYMBOLS {
            let captures = SYMBOL_RE.captures(symbol).unwrap();
            assert_eq!(&captures["symbol"], symbol);
        }
    }

    /// Whether the check digit of a CAS number is right: the digits before
    /// it, from right to left, weighted 1, 2, 3..., summed, modulo 10.
    fn cas_check_digit_is_valid(cas: &str) -> bool {
        let digits: Vec<u32> = cas.chars().filter_map(|c| c.to_digit(10)).collect();
        let Some((check_digit, rest)) = digits.split_last() else {
            return false;
        };
        let sum: u32 = rest
            .iter()
            .rev()
            .zip(1..)
            .map(|(digit, weight)| digit * weight)
            .sum();
        sum % 10 == *check_digit
    }

    #[test]
    fn test_cas_check_digit() {
        assert!(cas_check_digit_is_valid("7732-18-5"));
        assert!(cas_check_digit_is_valid("64-17-5"));
        assert!(!cas_check_digit_is_valid("7732-18-4"));
    }

    #[test]
    fn test_cmr_cas_entries_are_valid() {
        let codes = CMR_CODES;
        let mut seen = HashSet::new();
        for (cas, classification) in CMR_CAS {
            assert!(CAS_NUMBER_RE.is_match(cas), "{cas}: format");
            assert!(cas_check_digit_is_valid(cas), "{cas}: check digit");
            assert!(seen.insert(cas), "{cas}: duplicate");
            assert!(!classification.is_empty(), "{cas}: no classification");
            for code in classification.split(' ') {
                assert!(codes.contains(&code), "{cas}: unknown code {code:?}");
            }
        }
    }

    #[test]
    fn test_units_are_consistent() {
        let types = [
            "quantity",
            "temperature",
            "concentration",
            "molecular_weight",
        ];
        let mut ids = HashSet::new();
        let mut labels = HashSet::new();
        for (id, label, multiplier, unit_type, base) in UNITS {
            assert!(ids.insert(*id), "{label}: duplicate id");
            assert!(labels.insert(label), "{label}: duplicate label");
            assert!(types.contains(&unit_type), "{label}: type {unit_type}");
            assert!(
                multiplier.is_finite() && *multiplier > 0.0,
                "{label}: multiplier"
            );

            match base {
                // A base unit converts to itself.
                None => assert!((*multiplier - 1.0).abs() < f64::EPSILON, "{label}"),
                // The base unit exists, measures the same thing, and is itself
                // a base unit.
                Some(base_id) => {
                    let (_, base_label, _, base_type, base_base) = UNITS
                        .iter()
                        .find(|unit| *unit.0 == base_id)
                        .unwrap_or_else(|| panic!("{label}: unknown base unit {base_id}"));
                    assert_eq!(base_type, &unit_type, "{label}: base {base_label}");
                    assert!(base_base.is_none(), "{label}: base {base_label}");
                }
            }
        }
    }

    #[test]
    fn test_length_multipliers() {
        let multiplier = |label: &str| *UNITS.iter().find(|unit| unit.1 == label).unwrap().2;
        assert!((multiplier("dm") - 0.1).abs() < f64::EPSILON);
        assert!((multiplier("cm") - 0.01).abs() < f64::EPSILON);
        assert!((multiplier("mL") - 0.001).abs() < f64::EPSILON);
        assert!((multiplier("kg") - 1000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_periodic_table() {
        for symbol in PERIODIC_TABLE.keys() {
            let mut chars = symbol.chars();
            assert!(chars.next().unwrap().is_ascii_uppercase(), "{symbol}");
            assert!(chars.all(|c| c.is_ascii_lowercase()), "{symbol}");
            assert!(symbol.len() <= 2, "{symbol}");
        }
        // Deuterium is accepted in formulas.
        for symbol in ["H", "C", "O", "Cl", "D"] {
            assert!(PERIODIC_TABLE.contains_key(symbol), "{symbol}");
        }
    }

    #[test]
    fn test_statement_regexes() {
        let captures = HAZARD_STATEMENT_RE
            .captures("H300\tFatal if swallowed\t")
            .unwrap();
        assert_eq!(&captures["reference"], "H300");
        assert_eq!(&captures["label"], "Fatal if swallowed");
        let captures = HAZARD_STATEMENT_RE
            .captures("EUH066\tRepeated exposure may cause skin dryness\t")
            .unwrap();
        assert_eq!(&captures["reference"], "EUH066");

        for reference in ["H350i", "H360FD", "H300+H310+H330"] {
            let line = format!("{reference}\tlabel\t");
            let captures = HAZARD_STATEMENT_RE.captures(&line).unwrap();
            assert_eq!(&captures["reference"], reference);
        }
        // A reference must start the line and be followed by a tab.
        assert!(HAZARD_STATEMENT_RE.captures("xH300\tlabel\t").is_none());
        assert!(HAZARD_STATEMENT_RE.captures("H315+319\tlabel\t").is_none());

        for reference in ["P280", "P301+P330+P331", "P370+P380+P375[+P378]"] {
            let line = format!("{reference}\tlabel");
            let captures = PRECAUTIONARY_STATEMENT_RE.captures(&line).unwrap();
            assert_eq!(&captures["reference"], reference);
        }

        let captures = PRECAUTIONARY_STATEMENT_RE
            .captures("P301+P310\tIF SWALLOWED: Immediately call a doctor")
            .unwrap();
        assert_eq!(&captures["reference"], "P301+P310");
        assert_eq!(
            &captures["label"],
            "IF SWALLOWED: Immediately call a doctor"
        );
    }

    #[test]
    fn test_identifier_regexes() {
        assert!(CAS_NUMBER_RE.is_match("7732-18-5"));
        assert!(!CAS_NUMBER_RE.is_match("7732-18"));
        assert!(!CAS_NUMBER_RE.is_match("a7732-18-5"));
        assert!(CE_NUMBER_RE.is_match("200-578-6"));
        assert!(!CE_NUMBER_RE.is_match("200-5786"));
        assert!(ALL_ZERO_RE.is_match("0000"));
        assert!(!ALL_ZERO_RE.is_match("0010"));
        assert!(INCHI_REGEX.is_match("InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3"));
        assert!(!INCHI_REGEX.is_match("C2H6O"));
        assert!(INCHIKEY_REGEX.is_match("LFQSCWFLJHTTHZ-UHFFFAOYSA-N"));
        assert!(!INCHIKEY_REGEX.is_match("LFQSCWFLJHTTHZ-UHFFFAOYSA"));
    }

    #[test]
    fn test_request_regexes() {
        assert!(IDS_MATCH_RE.is_match("1,2,3"));
        assert!(IDS_MATCH_RE.is_match("12"));
        assert!(!IDS_MATCH_RE.is_match("1,a"));
        let ids: Vec<&str> = IDS_CAPTURE_RE
            .captures_iter("1,22,333")
            .map(|captures| captures.name("id").unwrap().as_str())
            .collect();
        assert_eq!(ids, ["1", "22", "333"]);

        let captures = END_OF_URL_CAPTURE_RE.captures("/products/12").unwrap();
        assert_eq!(&captures["id"], "12");
        assert!(END_OF_URL_CAPTURE_RE.captures("/products").is_none());
    }

    #[test]
    fn test_storage_barecode_regex() {
        let prefixes: Vec<&str> = STORAGE_BARECODE_RE
            .captures_iter("P3514.1,P3514.2,Box12.3")
            .map(|captures| captures.get(1).unwrap().as_str())
            .collect();
        assert_eq!(prefixes, ["P3514", "P3514", "Box12"]);
    }

    #[test]
    fn test_reference_regexes_match_the_ghs_file_regexes() {
        // A reference is valid iff the GHS file line regex accepts it.
        for reference in [
            "H350",
            "H350i",
            "H360FD",
            "EUH014",
            "H300+H310+H330",
            "H315+319",
            "H35",
            "h350",
        ] {
            assert_eq!(
                HAZARD_REFERENCE_RE.is_match(reference),
                HAZARD_STATEMENT_RE.is_match(&format!("{reference}\tlabel\t")),
                "{reference}"
            );
        }
        for reference in [
            "P280",
            "P301+P310",
            "P370+P380+P375[+P378]",
            "P30",
            "P301+",
            "p280",
        ] {
            assert_eq!(
                PRECAUTIONARY_REFERENCE_RE.is_match(reference),
                PRECAUTIONARY_STATEMENT_RE.is_match(&format!("{reference}\tlabel")),
                "{reference}"
            );
        }
        assert!(HAZARD_REFERENCE_RE.is_match("H300+H310"));
        assert!(!HAZARD_REFERENCE_RE.is_match("H300 H310"));
        assert!(PRECAUTIONARY_REFERENCE_RE.is_match("P301+P330+P331[+P310]"));
    }

    #[test]
    fn test_pubchem_code_regexes() {
        let codes = |regex: &Regex, text: &str| -> Vec<String> {
            regex
                .find_iter(text)
                .map(|code| code.as_str().to_string())
                .collect()
        };

        assert_eq!(
            codes(
                PUBCHEM_HAZARD_CODE_RE,
                "H302 (100%): Harmful; H300 + H310 and EUH014."
            ),
            ["H302", "H300 + H310", "EUH014"]
        );
        assert_eq!(
            codes(
                PUBCHEM_PRECAUTIONARY_CODE_RE,
                "P280, P301+P310 and P305 + P351."
            ),
            ["P280", "P301+P310", "P305 + P351"]
        );
        // Not a code inside a word.
        assert_eq!(codes(PUBCHEM_HAZARD_CODE_RE, "XH302").len(), 0);
    }

    #[test]
    fn test_html_color_regex() {
        assert!(HTML_COLOR_RE.is_match("#ff00AA"));
        for color in ["ff0000", "#ff00", "#gg0000", "#ff0000 "] {
            assert!(!HTML_COLOR_RE.is_match(color), "{color}");
        }
    }

    #[test]
    fn test_http_header_names() {
        // HTTP/2 requires lowercase header names.
        for header in [
            REQUEST_ID_HEADER,
            CHIMITHEQUE_PERSON_ID_HEADER,
            CHIMITHEQUE_PERSON_EMAIL_HEADER,
        ] {
            assert!(
                header.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                "{header}"
            );
        }
    }
}
