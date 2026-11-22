use super::*;

fn support() -> NativeSupport {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    NativeSupport::from_environment(&root).unwrap()
}

fn prose_plan(text: &str) -> plan::DocumentPlan {
    let graph = ftoo_manifest::seed().unwrap();
    let book = reader_fixture_book(&graph);
    let mut plan = fixture_plan_from_text(&book, text);
    let site = input::site("fixture.toml");
    plan.blocks[0].paragraph = true;
    plan.blocks.insert(
        0,
        plan::Block {
            pending_names: std::collections::BTreeMap::default(),
            displays: std::collections::BTreeMap::default(),
            unit: "sec:fixture:heading".into(),
            depth: Some(1),
            quotation: false,
            paragraph: false,
            site: site.clone(),
            runs: vec![plan::Inline::Text("A reading".into())],
        },
    );
    plan.screen = Some(plan::ScreenPlan {
        gaps: vec![
            plan::ScreenGap {
                position: GapPosition::Before,
                message: "Earlier content is unavailable.".into(),
                site: site.clone(),
                boundaries: vec![],
            },
            plan::ScreenGap {
                position: GapPosition::After,
                message: "Later content is unavailable.".into(),
                site,
                boundaries: vec![],
            },
        ],
        refrains: vec![],
        html_gaps: std::collections::BTreeMap::new(),
    });
    plan
}

fn math(site: UnitSite) -> plan::Inline {
    plan::Inline::Math {
        definition_site: site.clone(),
        key: "x".into(),
        markdown: "x".into(),
        latex: "x".into(),
        pending: vec![],
        site,
    }
}

#[test]
fn html_gap_planning_uses_nested_realizability() {
    let site = input::site("gap-plan.toml");
    let mut plan = prose_plan("Words.");
    plan.blocks[1].runs.push(math(site.clone()));
    assert!(plan::html_gaps(&plan).contains_key(&1));
    let mut admitted = math(site.clone());
    let plan::Inline::Math { markdown, .. } = &mut admitted else {
        unreachable!()
    };
    *markdown = COEFFICIENT.into();
    plan.blocks[1].runs = vec![admitted.clone()];
    assert!(plan::html_gaps(&plan).is_empty());
    plan.blocks[1].paragraph = false;
    assert!(plan::html_gaps(&plan).contains_key(&1));
    plan.blocks[1].paragraph = true;
    plan.blocks[1].runs = vec![plan::Inline::Styled {
        kind: plan::StyleKind::Emphasis,
        children: vec![admitted],
        child_sites: vec![site.clone()],
        site: site.clone(),
    }];
    assert!(plan::html_gaps(&plan).contains_key(&1));
    plan.blocks[1].runs = vec![plan::Inline::Styled {
        kind: plan::StyleKind::Emphasis,
        children: vec![plan::Inline::Mention {
            text: "code".into(),
            site: site.clone(),
        }],
        child_sites: vec![site.clone()],
        site,
    }];
    assert!(plan::html_gaps(&plan).is_empty());
}

#[test]
fn html_admission_refuses_gap_plan_disagreement_at_block() {
    let mut plan = prose_plan("Words.");
    let site = plan.blocks[1].site.clone();
    plan.blocks[1].runs.push(math(site.clone()));
    let error = super::super::html::prose_only(&plan).unwrap_err();
    assert!(format!("{error:?}").contains("checked HTML block realization and notice inventory"));
    let DocumentError::Assembly(errors) = error else {
        panic!("gap plan disagreement")
    };
    assert_eq!(
        errors.diagnostics()[0].site,
        AssemblySite::Source(site.clone())
    );
    let gaps = plan::html_gaps(&plan);
    plan.screen.as_mut().unwrap().html_gaps = gaps;
    plan.blocks[1].runs = vec![plan::Inline::Text("Words.".into())];
    let error = super::super::html::prose_only(&plan).unwrap_err();
    let DocumentError::Assembly(errors) = error else {
        panic!("gap plan disagreement")
    };
    assert_eq!(errors.diagnostics()[0].site, AssemblySite::Source(site));
}

#[test]
fn support_required_html_swapped_notice_and_block_refuses() {
    let support = support();
    let mut plan = prose_plan("First words.");
    let site = input::site("second-block.toml");
    plan.blocks.push(plan::Block {
        pending_names: std::collections::BTreeMap::new(),
        displays: std::collections::BTreeMap::new(),
        unit: "sec:fixture:second".into(),
        depth: None,
        quotation: false,
        paragraph: true,
        site: site.clone(),
        runs: vec![plan::Inline::Text("Second words.".into())],
    });
    plan.blocks[1].runs.push(math(site.clone()));
    let gaps = plan::html_gaps(&plan);
    let screen = plan.screen.as_mut().unwrap();
    screen.html_gaps = gaps;
    screen.gaps.insert(
        1,
        plan::ScreenGap {
            position: GapPosition::Between,
            message: "Middle content is unavailable.".into(),
            site,
            boundaries: vec![],
        },
    );
    let fonts = super::super::html::load_fonts(&plan, &support).unwrap();
    let rendered = super::super::html::render(&plan, &support, &[], None).unwrap();
    html_recovery::check(&plan, &fonts, &[], &[], &rendered).unwrap();
    let notice = "<p class=\"gap\">This passage is not yet available in this rendering.</p>";
    let swapped = rendered
        .replacen(notice, "<p>First words. x</p>", 1)
        .replacen("<p>Second words.</p>", notice, 1);
    let error = html_recovery::check(&plan, &fonts, &[], &[], &swapped).unwrap_err();
    assert!(format!("{error:?}").contains("HTML withheld block boundary"));
}

#[test]
fn support_required_html_prose_round_trip_and_mutations() {
    let support = support();
    let plan = prose_plan("See @emphasis{strong words} beside `dec:fixture:source`.");
    let fonts = super::super::html::load_fonts(&plan, &support).unwrap();
    let rendered =
        super::super::html::render(&plan, &support, &["dec:fixture:source"], None).unwrap();
    html_recovery::check(&plan, &fonts, &[], &["dec:fixture:source"], &rendered).unwrap();
    let paragraph_start = rendered.find("<p>See ").unwrap();
    let paragraph_end = paragraph_start + rendered[paragraph_start..].find("</p>").unwrap() + 4;
    let invented_notice = rendered.replacen(
        &rendered[paragraph_start..paragraph_end],
        "<p class=\"gap\">This passage is not yet available in this rendering.</p>",
        1,
    );
    let error = html_recovery::check(
        &plan,
        &fonts,
        &[],
        &["dec:fixture:source"],
        &invented_notice,
    )
    .unwrap_err();
    assert!(format!("{error:?}").contains("HTML paragraph boundary"));
    assert!(rendered.contains("<em>strong words</em>"));
    assert!(!rendered.contains("dec:fixture:source"));
    assert!(!rendered.contains("#missing-"));
    assert!(!rendered.contains("Unavailable in this rendering"));
    assert!(rendered.contains("font-family:'Pagella'"));
    assert!(!rendered.contains("font-family:&"));
    assert!(rendered.contains("<title>A reading</title>"));
    assert!(
        rendered
            .contains("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">")
    );
    assert!(rendered.contains("outline:3px solid #a34500"));
    let mut font_changed = rendered.as_bytes().to_vec();
    let face_at = rendered.find("base64,").unwrap() + "base64,".len();
    font_changed[face_at] = if font_changed[face_at] == b'A' {
        b'B'
    } else {
        b'A'
    };
    let font_changed = String::from_utf8(font_changed).unwrap();
    let licence_start = rendered
        .find("<pre data-file=\"GUST-FONT-LICENSE.txt\">")
        .unwrap();
    let licence_end =
        rendered[licence_start..].find("</pre>").unwrap() + licence_start + "</pre>".len();
    let licence_entry = &rendered[licence_start..licence_end];
    for (changed, aspect) in [
        (
            rendered.replacen("<em>strong words</em>", "<em>strong</em> words", 1),
            "HTML style boundary",
        ),
        (
            rendered.replacen(
                "<section id=\"coverage\"><p>",
                "<section id=\"coverage\"><p>Changed ",
                1,
            ),
            "HTML coverage declaration",
        ),
        (
            rendered.replacen(
                "<p class=\"gap\">Earlier content is unavailable.",
                "<p class=\"gap\">Earlier material is unavailable.",
                1,
            ),
            "HTML gap sentence",
        ),
        (font_changed, "HTML font byte identity"),
        (
            rendered.replacen("max-width:46rem", "max-width:47rem", 1),
            "HTML stylesheet layout and fallback",
        ),
        (
            rendered.replacen("'Pagella',serif;max-width", "'Other',serif;max-width", 1),
            "HTML stylesheet layout and fallback",
        ),
        (
            rendered.replacen(
                "<title>A reading</title>",
                "<title>Other reading</title>",
                1,
            ),
            "HTML title text",
        ),
        (
            rendered.replacen("width=device-width, initial-scale=1", "width=980", 1),
            "HTML viewport",
        ),
        (
            rendered.replacen("outline:3px solid #a34500", "outline:1px solid #a34500", 1),
            "HTML stylesheet layout and fallback",
        ),
        (
            rendered.replacen("preliminary version", "final version", 1),
            "HTML licence bytes",
        ),
        (
            rendered.replacen(licence_entry, "", 1),
            "HTML licence inventory",
        ),
        (
            rendered.replacen("font-family:'Pagella'", "font-family:'Other'", 1),
            "HTML font family or face declaration",
        ),
        (
            rendered.replacen("font-family:'Pagella'", "font-family:&#39;Pagella&#39;", 1),
            "HTML stylesheet raw text",
        ),
        (
            rendered.replacen("strong words", "dec:fixture:source", 1),
            "HTML reader label literal",
        ),
        (
            rendered.replacen("href=\"#content\"", "href=\"dec:fixture:source\"", 1),
            "HTML reader label literal",
        ),
        (
            rendered.replacen("A reading</title>", "A&#x27;reading</title>", 1),
            "HTML syntax",
        ),
        (
            rendered.replacen("<p class=\"gap\">", "<p class=\"gap\"/>", 1),
            "HTML syntax",
        ),
        (
            rendered.replacen("<html lang=\"en\">", "<html lang =\"en\">", 1),
            "HTML syntax",
        ),
    ] {
        assert_ne!(
            changed, rendered,
            "mutation must change the generated HTML: {aspect}"
        );
        let error = html_recovery::check(&plan, &fonts, &[], &["dec:fixture:source"], &changed)
            .unwrap_err();
        assert!(format!("{error:?}").contains(aspect), "{aspect}: {error:?}");
    }
    let mut absent_face = fonts;
    absent_face.0.pop();
    let error = html_recovery::check(&plan, &absent_face, &[], &["dec:fixture:source"], &rendered)
        .unwrap_err();
    assert!(format!("{error:?}").contains("HTML font inventory"));
}

#[test]
fn support_required_html_published_excerpt_content_and_faces() {
    let support = support();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let prepared = prepare_documents(&root, &support).unwrap();
    let artifact = |path: &str| {
        prepared
            .artifacts()
            .iter()
            .find(|artifact| artifact.path() == path)
            .unwrap()
            .bytes()
    };
    let paper = std::str::from_utf8(artifact("generated/documents/paper-pilot/en.html")).unwrap();
    let explainer =
        std::str::from_utf8(artifact("generated/documents/explainer-pilot/en.html")).unwrap();
    assert_eq!(paper.matches("@font-face").count(), 3);
    assert_eq!(explainer.matches("@font-face").count(), 3);
    assert!(paper.contains("the fact is the shadow the principle casts"));
    assert!(paper.contains("id=\"sec-crecord--limit\""));
    assert!(explainer.contains("The record can watch the hand’s tremor turning."));
    assert!(explainer.contains("href=\"../paper-pilot/en.html#sec-crecord--limit\""));
}

#[test]
fn support_required_html_nested_styles_and_quotation_destination() {
    let support = support();
    let mut plan = prose_plan("Words.");
    let site = input::site("fixture.toml");
    plan.blocks[1].runs = vec![plan::Inline::Styled {
        kind: plan::StyleKind::Strong,
        children: vec![
            plan::Inline::Text("A ".into()),
            plan::Inline::Styled {
                kind: plan::StyleKind::Emphasis,
                children: vec![plan::Inline::Text("B".into())],
                child_sites: vec![site.clone()],
                site: site.clone(),
            },
            plan::Inline::Mention {
                text: "token".into(),
                site: site.clone(),
            },
        ],
        child_sites: vec![site.clone(), site.clone(), site.clone()],
        site: site.clone(),
    }];
    for (kind, word) in [
        (plan::StyleKind::Smallcaps, "Small"),
        (plan::StyleKind::Regular, "Plain"),
    ] {
        plan.blocks[1].runs.push(plan::Inline::Styled {
            kind,
            children: vec![plan::Inline::Text(word.into())],
            child_sites: vec![site.clone()],
            site: site.clone(),
        });
    }
    plan.blocks.push(plan::Block {
        pending_names: std::collections::BTreeMap::default(),
        displays: std::collections::BTreeMap::default(),
        unit: "sec:fixture:quotation".into(),
        depth: None,
        quotation: true,
        paragraph: false,
        site: site.clone(),
        runs: vec![plan::Inline::Text("Closing words.".into())],
    });
    let screen = plan.screen.as_mut().unwrap();
    screen.gaps.push(plan::ScreenGap {
        position: GapPosition::After,
        message: "Later content is unavailable.".into(),
        site: site.clone(),
        boundaries: vec![],
    });
    screen.refrains.push(plan::ScreenRefrain {
        handle: "sec:record-limit".into(),
        bound_label: "sec:coda:record-limit".into(),
        fragment: "sec-crecord--limit".into(),
        quotation: Some(plan::ScreenQuotation {
            owner_document: "paper-pilot".into(),
            language: "en".into(),
            reader_path: "generated/documents/paper-pilot/en.md".into(),
            fragment: "sec-crecord--limit".into(),
            href: "../paper-pilot/en.md#sec-crecord--limit".into(),
            owner_site: site.clone(),
        }),
        site,
        binding_site: None,
    });
    let fonts = super::super::html::load_fonts(&plan, &support).unwrap();
    let mut selected =
        std::collections::BTreeMap::from([("paper-pilot".to_owned(), vec![DocumentOutput::Html])]);
    let mut owner = prose_plan("Owner words.");
    owner.document = "paper-pilot".into();
    owner.blocks[1].paragraph = false;
    owner
        .screen
        .as_mut()
        .unwrap()
        .refrains
        .push(plan::ScreenRefrain {
            handle: "sec:record-limit".into(),
            bound_label: "sec:coda:record-limit".into(),
            fragment: "sec-crecord--limit".into(),
            quotation: None,
            site: input::site("owner.toml"),
            binding_site: None,
        });
    super::super::html::check_destinations(&plan, std::slice::from_ref(&owner), &selected).unwrap();
    owner.blocks[1].runs.push(math(input::site("owner.toml")));
    let DocumentError::Assembly(errors) =
        super::super::html::check_destinations(&plan, std::slice::from_ref(&owner), &selected)
            .unwrap_err()
    else {
        panic!("withheld destination fault")
    };
    assert_eq!(
        errors.diagnostics()[0].site,
        AssemblySite::Source(plan.screen.as_ref().unwrap().refrains[0].site.clone())
    );
    let rendered = super::super::html::render(&plan, &support, &[], None).unwrap();
    assert!(rendered.contains("<strong>A <em>B</em><code>token</code></strong>"));
    assert!(
        rendered.contains(
            "code{font-family:'Inconsolata',monospace;font-weight:400;font-style:normal}"
        )
    );
    let synthesized_code = rendered.replacen(
        "code{font-family:'Inconsolata',monospace;font-weight:400;font-style:normal}",
        "code{font-family:'Inconsolata',monospace;font-weight:700;font-style:normal}",
        1,
    );
    assert_ne!(synthesized_code, rendered);
    let error = html_recovery::check(&plan, &fonts, &[], &[], &synthesized_code).unwrap_err();
    assert!(format!("{error:?}").contains("HTML monospace face"));
    assert!(rendered.contains("<span style=\"font-variant:small-caps\">Small</span>"));
    assert!(rendered.contains("<span style=\"font-weight:normal;font-style:normal\">Plain</span>"));
    let expected_href = "../paper-pilot/en.html#sec-crecord--limit";
    assert!(rendered.contains(expected_href));
    html_recovery::check(&plan, &fonts, &[], &[], &rendered).unwrap();
    let changed = rendered.replacen(expected_href, "../paper-pilot/en.html#sec-cother", 1);
    let error = html_recovery::check(&plan, &fonts, &[], &[], &changed).unwrap_err();
    assert!(format!("{error:?}").contains("HTML quotation href"));
    selected.clear();
    assert!(super::super::html::check_destinations(&plan, &[], &selected).is_err());
}

#[test]
fn html_math_adapter_absence_refuses_before_fonts() {
    let absent_support = NativeSupport::at(std::env::temp_dir().join("html-mathematics-no-fonts"));
    let plan = coefficient_plan();
    let error = super::super::html::render(&plan, &absent_support, &[], None).unwrap_err();
    let DocumentError::Assembly(errors) = error else {
        panic!("adapter absence must refuse before font loading")
    };
    let plan::Inline::Math { site, .. } = &plan.blocks[1].runs[1] else {
        unreachable!()
    };
    assert_eq!(
        errors.diagnostics()[0].site,
        AssemblySite::Source(site.clone())
    );
    assert!(format!("{errors:?}").contains("prepared notation adapter"));
}

const COEFFICIENT: &str = r"({\underbrace{{c}_{1}}_{\text{pending}}}={\frac{1}{3}})";
const SUBSCRIPT: &str = "{c}_{1}";

fn coefficient_plan() -> plan::DocumentPlan {
    let mut plan = prose_plan("A coefficient.");
    let site = input::site("coefficient.toml");
    let math = |source: &str, line: usize| plan::Inline::Math {
        definition_site: site.clone(),
        key: format!("math:{line}"),
        markdown: source.into(),
        latex: source.into(),
        pending: vec![],
        site: ftoo_manifest::UnitSite {
            occurrence: Some(line),
            ..site.clone()
        },
    };
    plan.blocks[1].runs = vec![
        plan::Inline::Text("The coefficient ".into()),
        math(COEFFICIENT, 11),
        plan::Inline::Text(" and subscript ".into()),
        math(SUBSCRIPT, 13),
        plan::Inline::Text(" appear here.".into()),
    ];
    plan
}

#[test]
fn html_math_library_envelope() {
    let source = "<span class=\"katex\"><math xmlns =\"http://www.w3.org/1998/Math/MathML\"><semantics><mrow><mi>c</mi><mspace /></mrow><annotation encoding=\"application/x-tex\">&#x27;</annotation></semantics></math></span>";
    let tree = html_recovery::library_fragment(source).unwrap();
    let mut canonical = String::new();
    super::super::html::serialize(&tree, &mut canonical);
    assert!(canonical.contains("&#39;"));
    assert!(canonical.contains("<mspace></mspace>"));
    assert!(html_recovery::library_fragment(&canonical).is_some());
    for changed in [
        source.replacen("class=\"katex\"", "class=\"katex\" class=\"katex\"", 1),
        format!("{source}trailing"),
    ] {
        assert_ne!(changed, source);
        assert!(html_recovery::library_fragment(&changed).is_none());
    }
    let page = include_str!("../../../../../registry/html-mathematics/h4.html");
    let start = page.find("<math xmlns=").unwrap();
    let end = start + page[start..].find("</math>").unwrap() + "</math>".len();
    let fragment = format!("<span class=\"katex\">{}</span>", &page[start..end]);
    let admitted = html_recovery::library_fragment(&fragment).unwrap();
    assert!(html_recovery::math_profile(&admitted, COEFFICIENT).is_some());
    let plan = coefficient_plan();
    let plan::Inline::Math { site, .. } = &plan.blocks[1].runs[1] else {
        unreachable!()
    };
    for changed in [
        fragment.replacen("</math></span>", "</math><math></math></span>", 1),
        fragment.replacen("class=\"katex\"", "class=\"katex\" role=\"math\"", 1),
        fragment.replacen("<math xmlns=", "<math display=\"block\" xmlns=", 1),
        fragment.replacen(
            "www.w3.org/1998/Math/MathML",
            "www.w3.org/1998/Math/Wrong",
            1,
        ),
        fragment.replacen(COEFFICIENT, SUBSCRIPT, 1),
        fragment.replacen("<mtext>pending</mtext>", "<mtext>absent</mtext>", 1),
    ] {
        assert_ne!(changed, fragment);
        let changed = html_recovery::library_fragment(&changed).unwrap();
        let error =
            super::super::html::checked_math_fragment(&changed, COEFFICIENT, site).unwrap_err();
        assert!(format!("{error:?}").contains("bounded MathML coefficient profile"));
        let DocumentError::Assembly(errors) = error else {
            panic!("profile fault must be located")
        };
        assert_eq!(
            errors.diagnostics()[0].site,
            AssemblySite::Source(site.clone())
        );
    }
}

#[test]
fn support_required_html_math_paragraph_round_trip() {
    let support = support();
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let catalogue = ftoo_manifest::notation(&book).unwrap();
    let notation = crate::notation::prepare_notation(&catalogue).unwrap();
    let plan = coefficient_plan();
    let prepared = super::super::html::prepare_math(&plan, Some(&notation)).unwrap();
    let fonts = super::super::html::load_fonts(&plan, &support).unwrap();
    let rendered = super::super::html::render(&plan, &support, &[], Some(&notation)).unwrap();
    assert!(
        rendered.as_bytes() == include_bytes!("../../../../../registry/html-mathematics/h4.html")
    );
    html_recovery::check(&plan, &fonts, &prepared, &[], &rendered).unwrap();
    let positions = [
        rendered.find("<mn>1</mn>").unwrap(),
        rendered.find("<mtext>pending</mtext>").unwrap(),
    ];
    for (changed, aspect) in [
        (
            rendered.replacen("<mn>1</mn>", "<mn>3</mn>", 1),
            "HTML mathematical presentation",
        ),
        (
            rendered.replacen("<mtext>pending</mtext>", "<mtext>absent</mtext>", 1),
            "HTML mathematical presentation",
        ),
        (
            rendered.replacen(COEFFICIENT, SUBSCRIPT, 1),
            "HTML mathematical annotation",
        ),
        (
            rendered.replacen("application/x-tex", "application/x-other", 1),
            "HTML mathematical presentation",
        ),
        (
            rendered.replacen("<math xmlns=", "<math display=\"block\" xmlns=", 1),
            "HTML mathematical presentation",
        ),
        (
            rendered.replacen("<math xmlns=", "<math xmlns=\"wrong\" xmlns=", 1),
            "HTML syntax",
        ),
    ] {
        assert_ne!(changed, rendered);
        let error = html_recovery::check(&plan, &fonts, &prepared, &[], &changed).unwrap_err();
        assert!(format!("{error:?}").contains(aspect), "{aspect}: {error:?}");
        if aspect.starts_with("HTML mathematical") {
            let DocumentError::Assembly(errors) = error else {
                panic!("mathematical source site")
            };
            let plan::Inline::Math { site, .. } = &plan.blocks[1].runs[1] else {
                unreachable!()
            };
            assert_eq!(
                errors.diagnostics()[0].site,
                AssemblySite::Source(site.clone())
            );
        }
    }
    assert!(positions[0] < positions[1]);
    let mut swapped = prepared.clone();
    swapped.swap(0, 1);
    let error = html_recovery::check(&plan, &fonts, &swapped, &[], &rendered).unwrap_err();
    assert!(format!("{error:?}").contains("HTML mathematical source occurrence"));
    let root_start = rendered.find("<math xmlns=").unwrap();
    let root_end = root_start + rendered[root_start..].find("</math>").unwrap() + "</math>".len();
    let root = &rendered[root_start..root_end];
    let next_start = root_end + rendered[root_end..].find("<math xmlns=").unwrap();
    let next_end = next_start + rendered[next_start..].find("</math>").unwrap() + "</math>".len();
    let next_root = &rendered[next_start..next_end];
    let swapped_page = rendered
        .replacen(root, "MATH_OCCURRENCE_SENTINEL", 1)
        .replacen(next_root, root, 1)
        .replacen("MATH_OCCURRENCE_SENTINEL", next_root, 1);
    assert_ne!(swapped_page, rendered);
    let error = html_recovery::check(&plan, &fonts, &prepared, &[], &swapped_page).unwrap_err();
    assert!(format!("{error:?}").contains("HTML mathematical annotation"));
    for (changed, aspect) in [
        (
            rendered.replacen(root, "", 1),
            "HTML inline text or identity",
        ),
        (
            rendered.replacen(root, &format!("{root}{root}"), 1),
            "HTML inline text or identity",
        ),
    ] {
        assert_ne!(changed, rendered);
        let error = html_recovery::check(&plan, &fonts, &prepared, &[], &changed).unwrap_err();
        assert!(format!("{error:?}").contains(aspect), "{aspect}: {error:?}");
    }
    let label_changed = rendered.replacen(
        "<mtext>pending</mtext>",
        "<mtext>dec:fixture:source</mtext>",
        1,
    );
    assert_ne!(label_changed, rendered);
    let error = html_recovery::check(
        &plan,
        &fonts,
        &prepared,
        &["dec:fixture:source"],
        &label_changed,
    )
    .unwrap_err();
    assert!(format!("{error:?}").contains("HTML reader label literal"));
    let DocumentError::Assembly(errors) = error else {
        panic!("mathematical label site")
    };
    let plan::Inline::Math { site, .. } = &plan.blocks[1].runs[1] else {
        unreachable!()
    };
    assert_eq!(
        errors.diagnostics()[0].site,
        AssemblySite::Source(site.clone())
    );
    let css_changed = rendered.replacen(
        "math mtext{font-family:'Pagella',serif}",
        "math mtext{font-family:'Other',serif}",
        1,
    );
    assert_ne!(css_changed, rendered);
    let error = html_recovery::check(&plan, &fonts, &prepared, &[], &css_changed).unwrap_err();
    assert!(format!("{error:?}").contains("HTML mathematical face"));
    let face_line = rendered
        .lines()
        .find(|line| line.starts_with("@font-face{font-family:'Pagella Math'"))
        .unwrap();
    for changed in [
        rendered.replacen(&format!("{face_line}\n"), "", 1),
        rendered.replacen(
            face_line,
            &face_line.replacen("Pagella Math", "Wrong", 1),
            1,
        ),
    ] {
        assert_ne!(changed, rendered);
        let error = html_recovery::check(&plan, &fonts, &prepared, &[], &changed).unwrap_err();
        assert!(format!("{error:?}").contains("HTML font family or face declaration"));
    }
    let mut absent_face = super::super::html::Fonts(fonts.0.clone());
    absent_face
        .0
        .retain(|(face, _)| face.family != "Pagella Math");
    let error = html_recovery::check(&plan, &absent_face, &prepared, &[], &rendered).unwrap_err();
    assert!(format!("{error:?}").contains("HTML font inventory"));
    let face_start = rendered
        .find("@font-face{font-family:'Pagella Math'")
        .unwrap();
    let encoded = face_start + rendered[face_start..].find("base64,").unwrap() + "base64,".len();
    let mut changed = rendered.as_bytes().to_vec();
    changed[encoded] = if changed[encoded] == b'A' { b'B' } else { b'A' };
    let changed = String::from_utf8(changed).unwrap();
    let error = html_recovery::check(&plan, &fonts, &prepared, &[], &changed).unwrap_err();
    assert!(format!("{error:?}").contains("HTML font byte identity"));
    let notice_start = rendered
        .find("<pre data-file=\"texgyrepagella-math.otf\">")
        .unwrap();
    let notice_end =
        notice_start + rendered[notice_start..].find("</pre>").unwrap() + "</pre>".len();
    let changed = rendered.replacen(&rendered[notice_start..notice_end], "", 1);
    let error = html_recovery::check(&plan, &fonts, &prepared, &[], &changed).unwrap_err();
    assert!(format!("{error:?}").contains("HTML licence inventory"));
    let missing_path =
        std::env::temp_dir().join(format!("ftoo-html-math-missing-{}", std::process::id()));
    std::fs::create_dir_all(missing_path.join("bundle")).unwrap();
    for file in ["texgyrepagella-regular.otf", "texgyrepagella-bold.otf"] {
        std::os::unix::fs::symlink(
            support.directory().join("bundle").join(file),
            missing_path.join("bundle").join(file),
        )
        .unwrap();
    }
    let missing = NativeSupport::at(missing_path.clone());
    let Err(error) = super::super::html::load_fonts(&plan, &missing) else {
        panic!("missing mathematics face must refuse")
    };
    let DocumentError::Assembly(errors) = error else {
        panic!("missing mathematics face")
    };
    let plan::Inline::Math { site, .. } = &plan.blocks[1].runs[1] else {
        unreachable!()
    };
    assert_eq!(
        errors.diagnostics()[0].site,
        AssemblySite::Source(site.clone())
    );
    std::fs::remove_dir_all(missing_path).unwrap();
}

#[test]
fn html_math_context_refusals() {
    let site = input::site("math-context.toml");
    let math = plan::Inline::Math {
        definition_site: site.clone(),
        key: "math:coefficient".into(),
        markdown: COEFFICIENT.into(),
        latex: COEFFICIENT.into(),
        pending: vec![],
        site: site.clone(),
    };
    for context in ["heading", "coverage", "display", "heading-display"] {
        let mut plan = prose_plan("Words.");
        match context {
            "heading" => plan.blocks[0].runs.push(math.clone()),
            "coverage" => plan.coverage.runs.push(math.clone()),
            "display" => {
                plan.blocks[1].displays.insert(
                    DocumentTarget::Latex,
                    plan::DisplayBody {
                        source: "fixture".into(),
                        layout: ftoo_manifest::DisplayLayout::Quote { rows: vec![] },
                        regions: vec![],
                        site: site.clone(),
                    },
                );
                plan.blocks[1].runs.push(math.clone());
            }
            "heading-display" => {
                plan.blocks[0].site = site.clone();
                plan.blocks[0].displays.insert(
                    DocumentTarget::Latex,
                    plan::DisplayBody {
                        source: "fixture".into(),
                        layout: ftoo_manifest::DisplayLayout::Quote { rows: vec![] },
                        regions: vec![],
                        site: site.clone(),
                    },
                );
            }
            _ => unreachable!(),
        }
        let absent = NativeSupport::at(std::env::temp_dir().join("html-math-context-no-fonts"));
        let error = super::super::html::render(&plan, &absent, &[], None).unwrap_err();
        assert!(
            format!("{error:?}").contains("checked HTML block realization and notice inventory"),
            "{context}: {error:?}"
        );
        let DocumentError::Assembly(errors) = error else {
            panic!("{context}: assembly fault")
        };
        assert_eq!(
            errors.diagnostics()[0].site,
            AssemblySite::Source(site.clone()),
            "{context}"
        );
    }
}

#[test]
fn html_math_untrusted_link_is_not_realizable() {
    let mut plan = coefficient_plan();
    let plan::Inline::Math { markdown, site, .. } = &mut plan.blocks[1].runs[1] else {
        unreachable!()
    };
    *markdown = r"\href{https://example.com}{c}".into();
    let site = site.clone();
    assert!(plan::html_gaps(&plan).contains_key(&1));
    let Err(error) = super::super::html::prepare_math(&plan, None) else {
        panic!("untrusted mathematical link must refuse")
    };
    assert!(format!("{error:?}").contains("realizable mathematical source"));
    let DocumentError::Assembly(errors) = error else {
        panic!("link refusal must be located")
    };
    assert_eq!(errors.diagnostics()[0].site, AssemblySite::Source(site));
}

#[test]
fn support_required_html_paper_key_exception_is_exact() {
    let support = support();
    let mut plan = prose_plan("Words.");
    let site = input::site("paper-key.toml");
    let evidence = ftoo_manifest::PaperEvidenceSite {
        file: "documents/paper-build/paper-tags.tsv".into(),
        line: 1,
    };
    plan.blocks[1].runs = vec![
        plan::Inline::Text("A ".into()),
        plan::Inline::PaperTag {
            value: ftoo_manifest::PaperTagValue {
                key: "src:nd".into(),
                spelling: "src:nd".into(),
                target: "node".into(),
                target_kind: "node".into(),
                lookup_site: evidence.clone(),
                alias_site: None,
                declaration_sites: vec![evidence],
            },
            site: site.clone(),
        },
        plan::Inline::Text(" span.".into()),
    ];
    let fonts = super::super::html::load_fonts(&plan, &support).unwrap();
    let rendered = super::super::html::render(&plan, &support, &["src:nd"], None).unwrap();
    html_recovery::check(&plan, &fonts, &[], &["src:nd"], &rendered).unwrap();
    for changed in [
        rendered.replacen("<title>A reading</title>", "<title>src:nd</title>", 1),
        rendered.replacen("id=\"content\"", "id=\"src:nd\"", 1),
        rendered.replacen("href=\"#content\"", "href=\"src:nd\"", 1),
        rendered.replacen(" span.", " src:nd span.", 1),
    ] {
        assert_ne!(changed, rendered);
        let error = html_recovery::check(&plan, &fonts, &[], &["src:nd"], &changed).unwrap_err();
        assert!(
            format!("{error:?}").contains("HTML reader label literal"),
            "{error:?}"
        );
    }
    plan.blocks[1].runs[2] = plan::Inline::Text("extra span.".into());
    let Err(error) = super::super::html::render(&plan, &support, &["src:nd"], None) else {
        panic!("paper key joined to text must refuse")
    };
    assert!(
        format!("{error:?}").contains("HTML reader label literal"),
        "{error:?}"
    );
    plan.blocks[1].runs[2] = plan::Inline::Text(" span.".into());
    let plan::Inline::PaperTag { value, .. } = &mut plan.blocks[1].runs[1] else {
        panic!("paper tag fixture")
    };
    value.key = "src:other".into();
    let Err(error) = super::super::html::render(&plan, &support, &["src:nd"], None) else {
        panic!("alias spelling must not receive the paper-key exception")
    };
    assert!(
        format!("{error:?}").contains("HTML reader label literal"),
        "{error:?}"
    );
}

#[test]
fn html_heading_reference_refuses_at_its_own_site() {
    let mut plan = prose_plan("Words.");
    let site = input::site("heading-reference.toml");
    plan.blocks[0].runs.push(plan::Inline::Tag {
        handle: "v:1".into(),
        label: "dec:fixture:source".into(),
        token: "v:1".into(),
        site: site.clone(),
    });
    let DocumentError::Assembly(errors) = super::super::html::prose_only(&plan).unwrap_err() else {
        panic!("heading reference fault")
    };
    assert_eq!(
        errors.diagnostics()[0].site,
        AssemblySite::Source(site.clone())
    );
    let math_site = input::site("heading-math.toml");
    plan.blocks[0].runs.insert(
        1,
        plan::Inline::Math {
            definition_site: math_site.clone(),
            key: "math:coefficient".into(),
            markdown: COEFFICIENT.into(),
            latex: COEFFICIENT.into(),
            pending: vec![],
            site: math_site.clone(),
        },
    );
    let DocumentError::Assembly(errors) = super::super::html::prose_only(&plan).unwrap_err() else {
        panic!("heading mathematics must precede a later reference fault")
    };
    assert_eq!(
        errors.diagnostics()[0].site,
        AssemblySite::Source(math_site)
    );
}

#[test]
fn support_required_html_font_inventory_and_pinned_bytes() {
    let support = support();
    let plan = prose_plan("Words.");
    let temporary = std::env::temp_dir().join(format!("html-font-{}", std::process::id()));
    let bundle = temporary.join("bundle");
    std::fs::create_dir_all(&bundle).unwrap();
    let copied = NativeSupport::at(temporary.clone());
    assert!(super::super::html::load_fonts(&plan, &copied).is_err());
    for file in ["texgyrepagella-regular.otf", "texgyrepagella-bold.otf"] {
        std::fs::copy(
            support.directory().join("bundle").join(file),
            bundle.join(file),
        )
        .unwrap();
    }
    super::super::html::load_fonts(&plan, &copied).unwrap();
    let regular = bundle.join("texgyrepagella-regular.otf");
    let mut bytes = std::fs::read(&regular).unwrap();
    bytes[0] ^= 1;
    std::fs::write(&regular, bytes).unwrap();
    assert!(super::super::html::load_fonts(&plan, &copied).is_err());
    std::fs::remove_dir_all(temporary).unwrap();
}

#[test]
fn support_required_html_withheld_mathematics_round_trip_and_mutations() {
    let support = support();
    let mut plan = prose_plan("Words.");
    let site = input::site("math-only.toml");
    plan.blocks[1].runs.push(plan::Inline::Math {
        definition_site: site.clone(),
        key: "x".into(),
        markdown: "x".into(),
        latex: "x".into(),
        pending: vec![],
        site: site.clone(),
    });
    plan.blocks[1].runs.push(plan::Inline::Mention {
        text: "hidden-code".into(),
        site: site.clone(),
    });
    assert!(!super::super::html::realizable(&plan.blocks[1].runs, true));
    plan.screen.as_mut().unwrap().html_gaps.insert(
        1,
        plan::ScreenGap {
            position: GapPosition::Between,
            message: "This passage is not yet available in this rendering.".into(),
            site,
            boundaries: vec![],
        },
    );
    let fonts = super::super::html::load_fonts(&plan, &support).unwrap();
    let rendered = super::super::html::render(&plan, &support, &[], None).unwrap();
    html_recovery::check(&plan, &fonts, &[], &[], &rendered).unwrap();
    let notice = "<p class=\"gap\">This passage is not yet available in this rendering.</p>";
    assert!(rendered.contains(notice));
    assert!(!rendered.contains("Words."));
    assert!(!rendered.contains("hidden-code"));
    assert!(!rendered.contains("Inconsolata"));
    assert!(!rendered.contains("math-only"));
    assert_eq!(plan.screen.as_ref().unwrap().html_gaps.len(), 1);
    let artifact = DocumentArtifact {
        path: "generated/documents/fixture/en.html".into(),
        bytes: rendered.as_bytes().to_vec(),
        origins: vec![],
        emissions: vec![],
        html_gap_sites: vec![input::site("math-only.toml")],
    };
    let DocumentError::Assembly(errors) =
        super::super::refuse_release_html_gaps(&[artifact]).unwrap_err()
    else {
        panic!("release gap fault")
    };
    assert_eq!(
        errors.diagnostics()[0].site,
        AssemblySite::Source(input::site("math-only.toml"))
    );
    let artifact = DocumentArtifact {
        path: "generated/documents/fixture/en.html".into(),
        bytes: rendered.as_bytes().to_vec(),
        origins: vec![],
        emissions: vec![],
        html_gap_sites: vec![input::site("math-only.toml")],
    };
    let release = crate::paper::release_html_gaps(&[artifact]).unwrap_err();
    assert!(
        release
            .to_string()
            .contains("Paper release M065 refused at HTML block gaps")
    );
    for (changed, fault) in [
        (
            rendered.replacen(notice, "", 1),
            "HTML withheld block sentence",
        ),
        (
            rendered.replacen(notice, "<p>Words. x</p>", 1),
            "HTML withheld block boundary",
        ),
        (
            rendered.replacen("not yet available", "currently unavailable", 1),
            "HTML withheld block sentence",
        ),
        (
            rendered.replacen(
                "<p class=\"gap\">Earlier content is unavailable.</p>",
                notice,
                1,
            ),
            "HTML gap sentence",
        ),
    ] {
        let error = html_recovery::check(&plan, &fonts, &[], &[], &changed).unwrap_err();
        assert!(format!("{error:?}").contains(fault), "{error:?}");
    }
}

#[test]
fn support_required_html_withheld_refrain_has_no_destination_or_handle() {
    let support = support();
    let mut plan = prose_plan("Refrain words.");
    let site = input::site("refrain-math.toml");
    plan.blocks[1].paragraph = false;
    plan.blocks[1].runs.push(plan::Inline::Math {
        definition_site: site.clone(),
        key: "x".into(),
        markdown: "x".into(),
        latex: "x".into(),
        pending: vec![],
        site: site.clone(),
    });
    let screen = plan.screen.as_mut().unwrap();
    screen.refrains.push(plan::ScreenRefrain {
        handle: "reading:handle".into(),
        bound_label: "sec:fixture:refrain".into(),
        fragment: "reading-handle".into(),
        quotation: None,
        site: site.clone(),
        binding_site: None,
    });
    screen.html_gaps.insert(
        1,
        plan::ScreenGap {
            position: GapPosition::Between,
            message: "This passage is not yet available in this rendering.".into(),
            site,
            boundaries: vec![],
        },
    );
    let rendered = super::super::html::render(&plan, &support, &[], None).unwrap();
    assert!(!rendered.contains("Refrain words."));
    assert!(!rendered.contains("reading-handle"));
    assert!(!rendered.contains("reading:handle"));
}

#[test]
fn support_required_html_withheld_owner_and_quotation_show_two_notices() {
    let support = support();
    let site = input::site("quoted-math.toml");
    let mut owner = prose_plan("Shared words.");
    owner.document = "owner".into();
    owner.blocks[1].paragraph = false;
    owner.blocks[1].unit = "sec:fixture:shared".into();
    owner.blocks[1].runs.push(math(site.clone()));
    owner
        .screen
        .as_mut()
        .unwrap()
        .refrains
        .push(plan::ScreenRefrain {
            handle: "shared".into(),
            bound_label: "sec:fixture:shared".into(),
            fragment: "shared".into(),
            quotation: None,
            site: site.clone(),
            binding_site: None,
        });
    owner.screen.as_mut().unwrap().html_gaps = plan::html_gaps(&owner);
    let mut quote = prose_plan("Shared words.");
    quote.document = "quote".into();
    quote.blocks[1].paragraph = false;
    quote.blocks[1].quotation = true;
    quote.blocks[1].unit = "sec:fixture:shared".into();
    quote.blocks[1].runs.push(math(site.clone()));
    quote
        .screen
        .as_mut()
        .unwrap()
        .refrains
        .push(plan::ScreenRefrain {
            handle: "shared".into(),
            bound_label: "sec:fixture:shared".into(),
            fragment: "shared".into(),
            quotation: Some(plan::ScreenQuotation {
                owner_document: "owner".into(),
                language: "en".into(),
                reader_path: "generated/documents/owner/en.md".into(),
                fragment: "shared".into(),
                href: "../owner/en.md#shared".into(),
                owner_site: site.clone(),
            }),
            site,
            binding_site: None,
        });
    quote.screen.as_mut().unwrap().html_gaps = plan::html_gaps(&quote);
    let mut selected = std::collections::BTreeMap::new();
    selected.insert("owner".into(), vec![ftoo_manifest::DocumentOutput::Html]);
    super::super::html::check_destinations(&quote, std::slice::from_ref(&owner), &selected)
        .unwrap();
    let owner_html = super::super::html::render(&owner, &support, &[], None).unwrap();
    let quote_html = super::super::html::render(&quote, &support, &[], None).unwrap();
    let notice = "<p class=\"gap\">This passage is not yet available in this rendering.</p>";
    assert_eq!(
        owner_html.matches(notice).count() + quote_html.matches(notice).count(),
        2
    );
    assert!(!quote_html.contains("../owner/en.html#shared"));
    assert!(!owner_html.contains("id=\"shared\""));

    quote.blocks[1].runs = vec![plan::Inline::Text("Shared words.".into())];
    quote.screen.as_mut().unwrap().html_gaps.clear();
    let error = super::super::html::check_destinations(&quote, &[owner], &selected).unwrap_err();
    let DocumentError::Assembly(errors) = error else {
        panic!("owner anchor fault")
    };
    assert_eq!(
        errors.diagnostics()[0].site,
        AssemblySite::Source(quote.screen.as_ref().unwrap().refrains[0].site.clone())
    );
}
