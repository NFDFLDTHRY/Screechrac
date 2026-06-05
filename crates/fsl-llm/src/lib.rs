#![allow(dead_code)]
//! fsl-llm — OFF-AXIS ENGINE: the inverted Oracle. Rule-based bootstrap, swappable for a real model WITHOUT changing any calling code (functions CALL it; it returns only element arrays; fresh context every call). The DEFAULT cognitive engine: a deterministic, stateless implementor
//! SPATIAL ROLE: OFF-AXIS ENGINE — the consulted cognitive engine; never on a cable, never holds context.
//! of `core::Oracle`. It NEVER calls a function and NEVER mutates state; it only reads
//! the freshly-assembled context handed to it and returns element arrays. All
//! decisions are made on STRUCTURE — pointability, injected stop tokens, topic/stance,
//! and the priority ordering — so there is NO regex anywhere.
//!
//! A model-backed Oracle (one whose weights are produced by the external, undefined
//! "cradle") plugs into the SAME trait; this deterministic engine is what makes the
//! whole system runnable and reproducible without one.

use fsl_core::*;

/// SPATIAL ROLE: OFF-AXIS ENGINE (applied) — the Facilitator domain cognition built on
/// the inverted Oracle (free-form → structured jobs, UNK clarification, preset evolution).
pub mod facilitator;

/// The deterministic engine. Holds no conversation state (only inert config), which
/// is what keeps the system deterministic: identical context ⇒ identical output.
#[derive(Clone, Debug, Default)]
pub struct DeterministicOracle;

impl Oracle for DeterministicOracle {
    /// PFC schema application: assign each claim the schema whose name token appears in
    /// the claim (token membership, not regex); else the first schema. Probability rises
    /// with stance magnitude. Reads the 4.1 schema library passed in context.
    fn interpret(&self, ctx: &PfcContext) -> Vec<InterpretationDraft> {
        let mut out = vec![];
        for c in ctx.claims {
            let toks = tokenize(&c.text);
            let schema = ctx.schemas.iter()
                .find(|s| { let st = tokenize(s); !st.is_empty() && st.iter().all(|w| toks.contains(w)) })
                .cloned()
                .unwrap_or_else(|| ctx.schemas.first().cloned().unwrap_or_default());
            let mag = c.stance.map(|s| s.unsigned_abs() as f32).unwrap_or(0.0);
            out.push(InterpretationDraft { claim: c.id, schema, probability: (0.5 + 0.1 * mag).min(1.0) });
        }
        out
    }

    /// F1 Locate: route each claim via the shared, regex-free structural rule (which
    /// also carries the framing degrade). The membrane applies these routings.
    fn route(&self, ctx: &RoutingContext) -> Vec<RoutingDraft> {
        ctx.claims.iter()
            .map(|c| RoutingDraft { claim: c.id, routing: structural_route(c, ctx.invalid_policy, ctx.invalid_pressure) })
            .collect()
    }

    /// OBS: a claim becomes OBS exactly when it can be pointed at (I-A1).
    fn observe(&self, ctx: &ObsContext) -> Vec<ObsDraft> {
        ctx.claims.iter().filter(|c| c.is_pointable()).map(|c| ObsDraft { claim: c.id }).collect()
    }

    /// DELTA — BOTH forms, detected structurally:
    /// • ObsVsObs: same topic, OPPOSED stance between an incoming OBS and an existing OBS.
    /// • ObsVsRule: an incoming OBS whose stance ≠ an expected rule's stance on its topic.
    fn diff(&self, ctx: &DeltaContext) -> Vec<DeltaDraft> {
        let mut out = vec![];
        for inc in ctx.incoming {
            let (it, is_) = match (inc.topic, inc.stance) { (Some(t), Some(s)) => (t, s), _ => continue };
            // form 1 — OBS vs OBS
            for ex in ctx.existing {
                if ex.claim == inc.id { continue; }
                if ex.topic == Some(it) {
                    if let Some(es) = ex.stance {
                        if (is_ as i32) * (es as i32) < 0 {
                            out.push(DeltaDraft { kind: DeltaKindDraft::ObsVsObs { a: inc.id, b: ex.claim } });
                        }
                    }
                }
            }
            // form 2 — OBS vs expected rule
            for r in ctx.expected {
                if r.topic == it && r.expected_stance != is_ {
                    out.push(DeltaDraft { kind: DeltaKindDraft::ObsVsRule { obs: inc.id, rule: r.rule } });
                }
            }
        }
        out
    }

    /// UNK: a non-pointable claim yields an explicit UNK specifying what would become
    /// OBS ("a pointer for X"). `resolves` is left for the membrane/cable to populate
    /// against specific DELTAs (so it is never silently fabricated here).
    fn unknowns(&self, ctx: &UnkContext) -> Vec<UnkDraft> {
        ctx.claims.iter().filter(|c| !c.is_pointable()).map(|c| UnkDraft {
            about_claim: Some(c.id),
            needs: vec![format!("a pointer for: {}", c.text)],
            would_become_obs: format!("OBS once '{}' can be pointed at", c.text),
            resolves: vec![],
        }).collect()
    }

    /// UNK decomposition: each missing requirement becomes one sub-UNK requirement.
    /// (The cable enforces the UnkPolicy budget/depth around this.)
    fn decompose(&self, ctx: &DecomposeContext) -> Vec<UnkDraft> {
        ctx.need.iter().map(|need| UnkDraft {
            about_claim: None,
            needs: vec![format!("sub-requirement: {}", need)],
            would_become_obs: format!("OBS for sub-requirement '{}'", need),
            resolves: vec![],
        }).collect()
    }

    /// Meaning Engine: one Transition over the exact five-set, STEERED by style (4.5)
    /// and the priority ORDER (4.2). Directive reorders priorities + permits action;
    /// Evaluative labels importance; Descriptive only describes. Weight = ledger size.
    fn meaning(&self, ctx: &MeaningContext) -> Vec<TransitionDraft> {
        // weight = ledger size, attenuated by degree-of-separation (progressively lower
        // priority for deeper onion shells).
        let weight = ctx.ledger.len() as f32 / (1.0 + ctx.root.degree as f32);
        let top: Vec<String> = ctx.priority_order.iter().take(2).map(|s| s.to_string()).collect();
        let draft = match ctx.style {
            MeaningStyleTag::Directive => TransitionDraft {
                beliefs: vec!["situation is actionable".into()],
                priorities: top,                                   // reorders the stack (directive)
                roles: vec!["agent".into()],
                future_expectations: vec!["act on the crossing".into()],
                allowed_behaviors: vec!["directive".into()],       // permits behavior
                weight,
            },
            MeaningStyleTag::Evaluative => TransitionDraft {
                beliefs: vec!["this is good/bad".into()],
                priorities: vec![],
                roles: vec![],
                future_expectations: vec![],
                allowed_behaviors: vec![],                         // labels, does not direct → may stall
                weight,
            },
            MeaningStyleTag::Descriptive => TransitionDraft {
                beliefs: vec!["this is what happened".into()],
                priorities: vec![],
                roles: vec![],
                future_expectations: vec![],
                allowed_behaviors: vec![],
                weight,
            },
        };
        vec![draft]
    }
}

/// SPATIAL ROLE: OFF-AXIS ENGINE alias. The bootstrap rule-based engine, named for its
/// eventual role. Swapping in a real model means replacing this type's `impl Oracle`
/// body — no calling code changes (functions still CALL it; it still returns only
/// element arrays from freshly assembled context).
pub type RuleBasedLlm = DeterministicOracle;
