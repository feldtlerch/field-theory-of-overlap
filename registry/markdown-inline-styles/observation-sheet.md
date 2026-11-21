# Inline-style observation sheet

The candidate pages were viewed signed out in the public forge file view at commit `f88f7a11828ef50cd585456173bfe6c8add93670`. The rendered article element was read with anchor links and descriptive paragraphs omitted and `dir="auto"` attributes removed. The structures below are received element structures, not raw Markdown bytes or a font inventory. Only the proposed delimiters and identity comments follow the writer's bytes; surrounding fixture text can differ because the writer encodes ASCII punctuation as numeric entities. Admission is narrower than rendering: ADR 022 refuses styled mathematics, the unit style syntax refuses nesting, and display cases S25–S26 were not viewed.

The mention writer entity-escapes every ASCII punctuation character except comma, period, colon, semicolon, question mark and exclamation mark; it would emit `&#92;` for the backslash in S06. S06 records the candidate page's observed structure, not the writer's bytes, and the mention scanner refuses that backslash.

| Candidate page | Candidate commit | SHA-256 before viewing |
| --- | --- | --- |
| [basic faces](candidates/01-basic.md) | `f88f7a11828ef50cd585456173bfe6c8add93670` | `7b4283ad497dd257e096c97461745317c14a9251366529b6ebf8a40ff7722e40` |
| [boundaries](candidates/02-boundaries.md) | `f88f7a11828ef50cd585456173bfe6c8add93670` | `f509b2ec2f0167ed3ea11e1b3d6c4d722bb40243252ceb0a5deb38bc36d0023a` |
| [identity comments and mathematics](candidates/03-contexts.md) | `f88f7a11828ef50cd585456173bfe6c8add93670` | `b2c433c9cd1952ff7c6ca021b063a1071f6cebdd9658713ecce615083d20475d` |
| [structural contexts](candidates/04-structural.md) | `f88f7a11828ef50cd585456173bfe6c8add93670` | `0525ffff266676393d1da596bd04e3b6454284f6a5c0d0e1b1975c974dd614f2` |

| Case | Received element structure | Rendering verdict | Admission |
| --- | --- | --- | --- |
| S01 | `<p><em>could</em> move through this edge.</p>` | Faithful emphasis | Emphasis in ordinary prose |
| S02 | `<p><strong>capacity</strong>: how much could move.</p>` | Faithful strong weight | Strong in ordinary prose |
| S03 | Table with one header cell `Comparison` and one body cell `<td><em>what could move / what did move</em></td>` | Faithful style in cell | Refused while table structure is outside this style admission |
| S04 | `<h4 tabindex="-1" class="heading-element"><span>A Field Theory of Overlap</span></h4>`; `style` removed | Unfaithful small capitals | Refused |
| S05 | `<p>A <code>nec</code> subsection.</p>` | Faithful code mention | Existing mention path |
| S06 | `<p><code>\RegisterVariance</code> supplies variance heads.</p>` | Faithful literal code element | Mention parser must first admit backslash |
| S07 | `<p><strong>capacity</strong>: how much <em>could</em> move.</p>` | Faithful runs separated by text | Strong and emphasis in prose with intervening text |
| S08 | `<p><strong>felt <em>total</em></strong> remains.</p>` | Faithful constructed nesting | Refused authored nesting |
| S09 | `<p><em>a <strong>fact</strong></em> remains.</p>` | Faithful constructed nesting | Refused authored nesting |
| S10 | `<p><em>helped</em>, <em>untouched</em>, or <em>harmed</em>.</p>` | Faithful commas outside emphasis | Emphasis in prose |
| S11 | `<p><em>And must — once the sufferer's own weighing is in hand</em>: the total is built on no shared rod.</p>` | Faithful colon outside emphasis | Emphasis in prose |
| S12 | `<p>What the event cost or paid <em>you, by your own caring</em>. This is the quiet marvel.</p>` | Faithful period outside emphasis | Emphasis in prose |
| S13 | `<p><strong>non-dimensional source</strong> src:nd, recovered never.</p>` | Faithful strong beside invisible label comment | Strong and tag in prose with intervening space |
| S14 | `<p><em>De gustibus</em> a:degustibus forbids exactly this.</p>` | Faithful emphasis beside invisible cite comment | Emphasis and citation in prose with intervening space |
| S15 | `<p><em>Existence</em> gen:w-exists: there is an input.</p>` | Faithful emphasis beside invisible label comment | Emphasis and tag in prose with intervening space |
| S16 | `<p><em>word src:nd</em>; <em>word a:degustibus</em>.</p>` | Faithful constructed comments inside emphasis | Comment-bearing children may retain identity |
| S17 | `<p><em>instrument</em>: <math-renderer class="js-inline-math" ...>$R$</math-renderer> cannot read a rate at rest.</p>` | Faithful style beside mathematics | Mathematics remains governed by ADR 022 |
| S18 | `<td><strong><math-renderer ...>$g$</math-renderer> (the on/off)</strong></td>` | Mathematics visibly rendered inside strong | Styled mathematics refused by ADR 022 |
| S19 | `<p><em><math-renderer ...>$i$</math-renderer>'s</em> counterfactual defector.</p>` | Mathematics visibly rendered inside emphasis | Styled mathematics refused by ADR 022 |
| S20 | First table header row `<th><span>register</span></th><th><span>tags</span></th><th><span>gloss</span></th>`; `style` removed | Unfaithful regular weight; default header weight remains | Refused |
| S21 | `<p>The anchor-<em>scarce</em> corner.</p>` | Faithful hyphen before emphasis | Emphasis in prose |
| S22 | `<p>The <em>self</em>-believed report.</p>` | Faithful hyphen after emphasis | Emphasis in prose |
| S23 | `<ul><li><strong>Ambivalence</strong>: real stake, but the components cancel.</li></ul>` | Faithful strong in list item | Refused while list structure is outside this style admission |
| S24 | Second table header row `<th><strong>the plain rule</strong></th><th><strong>why it matters</strong></th>` | Faithful strong in table header | Refused while table structure is outside this style admission |
| S25 | No received structure; display case not viewed | Unobserved | Refused pending display admission |
| S26 | No received structure; display case not viewed | Unobserved | Refused pending display admission |
