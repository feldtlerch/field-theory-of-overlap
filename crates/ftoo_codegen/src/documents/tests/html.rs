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
    plan.blocks[1].runs = vec![plan::Inline::Styled {
        kind: plan::StyleKind::Emphasis,
        children: vec![math(site.clone())],
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
    assert!(format!("{error:?}").contains("prose-only HTML document"));
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
    let rendered = super::super::html::render(&plan, &support, &[]).unwrap();
    html_recovery::check(&plan, &fonts, &[], &rendered).unwrap();
    let notice = "<p class=\"gap\">This passage is not yet available in this rendering.</p>";
    let swapped = rendered
        .replacen(notice, "<p>First words. x</p>", 1)
        .replacen("<p>Second words.</p>", notice, 1);
    let error = html_recovery::check(&plan, &fonts, &[], &swapped).unwrap_err();
    assert!(format!("{error:?}").contains("HTML withheld block boundary"));
}

#[test]
fn support_required_html_prose_round_trip_and_mutations() {
    let support = support();
    let plan = prose_plan("See @emphasis{strong words} beside `dec:fixture:source`.");
    let fonts = super::super::html::load_fonts(&plan, &support).unwrap();
    let rendered = super::super::html::render(&plan, &support, &["dec:fixture:source"]).unwrap();
    html_recovery::check(&plan, &fonts, &["dec:fixture:source"], &rendered).unwrap();
    let paragraph_start = rendered.find("<p>See ").unwrap();
    let paragraph_end = paragraph_start + rendered[paragraph_start..].find("</p>").unwrap() + 4;
    let invented_notice = rendered.replacen(
        &rendered[paragraph_start..paragraph_end],
        "<p class=\"gap\">This passage is not yet available in this rendering.</p>",
        1,
    );
    let error =
        html_recovery::check(&plan, &fonts, &["dec:fixture:source"], &invented_notice).unwrap_err();
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
    ] {
        assert_ne!(
            changed, rendered,
            "mutation must change the generated HTML: {aspect}"
        );
        let error =
            html_recovery::check(&plan, &fonts, &["dec:fixture:source"], &changed).unwrap_err();
        assert!(format!("{error:?}").contains(aspect), "{aspect}: {error:?}");
    }
    let mut absent_face = fonts;
    absent_face.0.pop();
    let error =
        html_recovery::check(&plan, &absent_face, &["dec:fixture:source"], &rendered).unwrap_err();
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
    owner.blocks[1].runs.push(plan::Inline::Math {
        definition_site: input::site("owner.toml"),
        key: "x".into(),
        markdown: "x".into(),
        latex: "x".into(),
        pending: vec![],
        site: input::site("owner.toml"),
    });
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
    let rendered = super::super::html::render(&plan, &support, &[]).unwrap();
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
    let error = html_recovery::check(&plan, &fonts, &[], &synthesized_code).unwrap_err();
    assert!(format!("{error:?}").contains("HTML monospace face"));
    assert!(rendered.contains("<span style=\"font-variant:small-caps\">Small</span>"));
    assert!(rendered.contains("<span style=\"font-weight:normal;font-style:normal\">Plain</span>"));
    let expected_href = "../paper-pilot/en.html#sec-crecord--limit";
    assert!(rendered.contains(expected_href));
    html_recovery::check(&plan, &fonts, &[], &rendered).unwrap();
    let changed = rendered.replacen(expected_href, "../paper-pilot/en.html#sec-cother", 1);
    let error = html_recovery::check(&plan, &fonts, &[], &changed).unwrap_err();
    assert!(format!("{error:?}").contains("HTML quotation href"));
    selected.clear();
    assert!(super::super::html::check_destinations(&plan, &[], &selected).is_err());
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
    assert!(!super::super::html::realizable(&plan.blocks[1].runs));
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
    let rendered = super::super::html::render(&plan, &support, &[]).unwrap();
    html_recovery::check(&plan, &fonts, &[], &rendered).unwrap();
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
        let error = html_recovery::check(&plan, &fonts, &[], &changed).unwrap_err();
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
    let rendered = super::super::html::render(&plan, &support, &[]).unwrap();
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
    let owner_html = super::super::html::render(&owner, &support, &[]).unwrap();
    let quote_html = super::super::html::render(&quote, &support, &[]).unwrap();
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
    let rendered = super::super::html::render(&plan, &support, &["src:nd"]).unwrap();
    html_recovery::check(&plan, &fonts, &["src:nd"], &rendered).unwrap();
    for changed in [
        rendered.replacen("<title>A reading</title>", "<title>src:nd</title>", 1),
        rendered.replacen("id=\"content\"", "id=\"src:nd\"", 1),
        rendered.replacen("href=\"#content\"", "href=\"src:nd\"", 1),
        rendered.replacen(" span.", " src:nd span.", 1),
    ] {
        assert_ne!(changed, rendered);
        let error = html_recovery::check(&plan, &fonts, &["src:nd"], &changed).unwrap_err();
        assert!(
            format!("{error:?}").contains("HTML reader label literal"),
            "{error:?}"
        );
    }
    plan.blocks[1].runs[2] = plan::Inline::Text("extra span.".into());
    let Err(error) = super::super::html::render(&plan, &support, &["src:nd"]) else {
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
    let Err(error) = super::super::html::render(&plan, &support, &["src:nd"]) else {
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
    assert_eq!(errors.diagnostics()[0].site, AssemblySite::Source(site));
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
