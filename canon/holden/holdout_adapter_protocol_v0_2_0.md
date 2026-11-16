# The Holdout Adapter Protocol v0.2.0
**Canonical adapter family for Holdout Skill v2.1.1**

**Executive Summary**

The **Holdout Adapter** is the canonical integration boundary between the frozen **Holdout Skill v2.1.1** core and any native orchestration, planning, tracking, storage, or runtime system. It exists because the Holdout Skill is intentionally stable where orchestration systems are intentionally unstable: filenames change, front matter evolves, status models differ, storage backends vary, dispatch patterns multiply, and runtime provenance becomes richer over time. The adapter absorbs that variation without weakening the core skill boundary.

The Holdout Adapter is **not** a second evaluator, **not** a planning doctrine, and **not** a replacement for `orchestration_integration`. It is a typed protocol family that tells an implementation:
1. how to ingest native artifacts,
2. how to normalize them into canonical Holdout inputs,
3. how to render canonical Holdout outputs into native artifacts,
4. how to carry dispatch/runtime provenance,
5. how to preserve security, redaction, and publication boundaries, and
6. how to validate round-trips and future schema evolution.

The governing rule is inherited from the Holdout Skill: if the adapter says something is mandatory, it must be typed and validated. The adapter therefore freezes the **translation seam**, not the user’s local workflow.

## Section 0: PREAMBLE

### 0.1 What the Adapter Is
The Holdout Adapter is a **portable translation boundary** that maps unstable native orchestration systems into the stable canonical Holdout artifact spine.

It is responsible for:
- ingesting native planning and execution artifacts,
- classifying them by stable semantics rather than local naming,
- projecting them into canonical Holdout inputs,
- rendering Holdout outputs into native work items, packets, rows, or files,
- attaching runtime and storage provenance,
- enforcing security and publication policy at the integration seam.

- [ADP-001] The adapter **MUST** preserve the Holdout Skill boundary: input contract in, typed artifacts out.
- [ADP-002] The adapter **MUST NOT** redefine Holdout doctrine, core artifact semantics, or canonical taxonomy.
- [ADP-003] The adapter **MUST** be able to target more than one orchestration/storage style without requiring a fork of the Holdout core.

### 0.2 What the Adapter Is Not
The adapter is not:
- a new planning doctrine,
- a replacement for the Holdout Skill,
- a runtime-specific plugin hard-coded into the core registry,
- a self-directing planner,
- or a reason to collapse `builder_packet` into `rendered_work_order`.

- [ADP-004] The adapter **MUST NOT** turn the Holdout Skill into a self-scheduling or self-directing agent.
- [ADP-005] The adapter **MUST NOT** hard-code a single work-order format into the Holdout core.
- [ADP-006] The adapter **MUST NOT** merge `builder_packet` with `rendered_work_order`.

### 0.3 Why the Adapter Exists
Planning is mutable. Sequencing may be outside-in or inside-out. Work may be file-native today and database-backed tomorrow. Runtime coordination may be single-agent, sequential DEV→QA, parallel wave dispatch, queue-driven, graph-driven, or saga-like. The adapter exists so those differences do not destabilize Holdout’s core.

- [ADP-007] The adapter **MUST** model planning as a revision-capable surface, not as a single frozen artifact.
- [ADP-008] The adapter **MUST** support both file-native and database- or API-backed native systems.
- [ADP-009] The adapter **MUST** treat dispatch pattern, storage pattern, and native artifact taxonomy as adapter concerns, not core concerns.

## Section 1: GOVERNING PRINCIPLES

### 1.1 Semantic Stability Over Local Conventions
Local conventions are allowed to drift. The adapter therefore keys off **stable semantics**:
- strategy / epic / story / slice as planning roles,
- dispatch wave / loop / handoff as coordination roles,
- evidence / review / handoff as governance roles,
- and canonical Holdout artifacts as evaluation roles.

- [ADP-010] The adapter **MUST** classify native artifacts by semantic role before mapping them.
- [ADP-011] Filename, folder, or front matter conventions **MAY** vary, but the semantic mapping **MUST** remain explicit.

### 1.2 Core Frozen, Adapter Flexible
The Holdout Skill v2.1.1 core remains the canonical evaluator. The adapter may extend integration surfaces, but it may not silently promote those extensions into the core registry.

- [ADP-012] Adapter-local schema families **MUST** live outside the core registry until promotion criteria are met.
- [ADP-013] Any proposal to move adapter-local schemas into the core **MUST** require a validation report and Ring 3 review.

### 1.3 Typed Round-Trip Honesty
A translation that cannot round-trip is either lossy, partial, or dishonest. The adapter must explicitly declare where information is preserved, normalized, dropped, or synthesized.

- [ADP-014] The adapter **MUST** declare lossy mappings.
- [ADP-015] The adapter **MUST** support round-trip validation for at least one golden fixture per mapping profile.
- [ADP-016] Unknown native fields **MUST** be preserved or explicitly classified as discarded.

### 1.4 Restrained Recursive Backpressure
Recursive evaluation is allowed at the adapter seam, but it must not dominate planners or create ceremonial noise.

- [ADP-017] The default recursive hook policy **MUST** fire at typed revision boundaries, before irreversible side effects, at fan-out/fan-in checkpoints, and at convergence decisions.
- [ADP-018] The adapter **MUST NOT** default to evaluating every planner step.
- [ADP-019] Recursive hook profiles **MAY** support broader coverage, but those profiles **MUST** remain explicitly selectable.

### 1.5 Public Safety and Publication Discipline
The adapter is an open-source component candidate. It must therefore be safe to publish and safe to embed.

- [ADP-020] The adapter **MUST** support redaction and retention policy at the schema level.
- [ADP-021] The adapter **MUST NOT** auto-execute rendered outputs, commands, or hooks merely because they are present in an artifact.
- [ADP-022] The adapter **MUST** distinguish publishable release evidence from private runtime residue.

## Section 2: PROTOCOL FAMILY

The canonical adapter family contains ten contracts.

### 2.1 adapter_manifest
The top-level adapter declaration.

```yaml
adapter_manifest:
  adapter_id: string
  adapter_name: string
  adapter_version: string
  canonical_holdout_version: string
  schema_registry_version: string
  native_system:
    system_name: string
    system_version: string
    deployment_mode: manual | local | ci | swarm | remote
    repo_root: string
    workspace_root: string
  supported_storage_backends: [string]
  supported_dispatch_patterns: [string]
  supported_render_targets: [string]
  compatibility_notes: [string]
```

- [ADP-023] Every adapter **MUST** declare an `adapter_manifest`.
- [ADP-024] `canonical_holdout_version` and `schema_registry_version` **MUST** be present on the manifest.

### 2.2 planning_surface_profile
The semantic description of native planning artifacts.

```yaml
planning_surface_profile:
  native_planning_types: [string]
  strategy_type_name: optional string
  epic_type_name: optional string
  story_type_name: optional string
  slice_type_name: optional string
  task_type_name: optional string
  review_type_names: [string]
  evidence_type_names: [string]
  decision_filter_type_name: optional string
  status_groups: [string]
  phase_groups: [string]
  front_matter_format: yaml | toml | json | none
  body_section_extractors:
    - section_name: string
      extraction_rule: string
```

- [ADP-025] The adapter **MUST** declare how native planning artifacts are recognized.
- [ADP-026] The adapter **MUST NOT** assume a single story/epic/slice vocabulary.

### 2.3 artifact_mapping_profile
The declarative bridge between native artifacts and Holdout artifacts.

```yaml
artifact_mapping_profile:
  native_to_canonical:
    - native_type: string
      canonical_role: string
      cardinality: one_to_one | one_to_many | many_to_one
      required_fields: [string]
      field_map: {}
      enum_map: {}
      section_extractors: {}
      loss_policy: preserve | normalize | drop | synthesize
  canonical_to_native:
    - canonical_type: string
      native_target: string
      renderer: string
      required_native_fields: [string]
      field_map: {}
      enum_map: {}
      loss_policy: preserve | normalize | drop | synthesize
```

- [ADP-027] The adapter **MUST** declare both inbound and outbound mappings.
- [ADP-028] Every mapping **MUST** state whether it is one-to-one, one-to-many, or many-to-one.

### 2.4 dispatch_profile
The coordination model.

```yaml
dispatch_profile:
  dispatch_patterns:
    - sequential
    - parallel
    - wave
    - handoff
    - iterative
    - queue
    - saga
  runtime_fields:
    - wave_id
    - wave_index
    - parallel_group_id
    - dispatch_order
    - convoy_id
    - fleet_id
    - loop_id
    - loop_iteration
    - assigned_agent_id
    - handoff_from
    - handoff_to
    - checkpoint_policy
    - retry_policy
    - timeout_policy
    - escalation_policy
```

- [ADP-029] The adapter **MUST** declare which dispatch patterns the native system actually uses.
- [ADP-030] The adapter **MUST** capture wave / loop / handoff metadata when such patterns exist.

### 2.5 storage_profile
The storage archetype and truth surface.

```yaml
storage_profile:
  backends:
    - backend_id: string
      backend_type: markdown | yaml | json | jsonl | sqlite | postgres | api | hybrid
      root_or_dsn: string
      key_columns: [string]
      json_paths: [string]
      append_only: boolean
      cache_layer: optional string
      lock_strategy: optional string
      merge_strategy: optional string
      watch_strategy: optional string
```

- [ADP-031] The adapter **MUST** declare the storage backends it depends on.
- [ADP-032] The adapter **MUST** identify which backend is authoritative for reads, writes, or cache hydration.

### 2.6 runtime_provenance_profile
The execution envelope.

```yaml
runtime_provenance_profile:
  capture_fields:
    - provider
    - model
    - model_version
    - harness
    - session_id
    - agent_id
    - fleet_id
    - container_id
    - vm_id
    - machine_id
    - ip_address
    - worktree_id
    - branch_name
    - base_sha
    - head_sha
    - workspace_path
```

- [ADP-033] The adapter **MUST** carry runtime provenance separately from human-readable planning artifacts.
- [ADP-034] The adapter **SHOULD** capture enough host and session metadata to reconstruct the execution environment.

### 2.7 render_profile
The adapter-only output layer.

```yaml
render_profile:
  native_work_order_type: string
  template_ref: string
  output_path_rules: {}
  naming_convention: string
  required_front_matter: {}
  required_sections: [string]
  required_hooks: [string]
  required_memories: [string]
  status_rendering_rules: {}
```

- [ADP-035] `builder_packet` **MUST** remain portable; orchestration-specific rendering belongs only in `render_profile`.
- [ADP-036] The adapter **MUST** be able to emit `rendered_work_order` without mutating the core Holdout packet schema.

### 2.8 security_and_publication_policy
The safe publication surface.

```yaml
security_and_publication_policy:
  safe_yaml_only: boolean
  no_auto_execute_rendered_outputs: boolean
  path_allowlist: [string]
  redaction_rules: [string]
  retention_defaults:
    standard: string
    audit: string
    legal_hold: string
  pii_risk_policy: string
  publishable_self_eval_policy: string
  private_runtime_policy: string
```

- [ADP-037] The adapter **MUST** declare redaction and retention defaults.
- [ADP-038] The adapter **MUST NOT** treat work orders, packets, or YAML artifacts as executable instructions without an explicit execution layer.

### 2.9 recursive_bridge_schemas
The planner-facing overlay.

```yaml
recursive_bridge_schemas:
  enabled: boolean
  default_hook_profile: revision_boundary_safe
  supported_hook_profiles:
    - revision_boundary_safe
    - holdout_first
    - traditional_gate
    - full_cycle_experimental
```

This family includes:
- `plan_revision_event`
- `loop_state`
- `execution_target`
- `feedback_artifact`
- `recursive_evaluation_hook`
- `plan_convergence_gate`

- [ADP-039] Recursive bridge schemas **MUST** remain adapter-local by default.
- [ADP-040] Any adapter exposing recursive bridge schemas **MUST** expose the selected hook profile explicitly.

### 2.10 round_trip_validation_profile
The anti-drift contract.

```yaml
round_trip_validation_profile:
  schema_validation_mode: strict | permissive
  round_trip_required: boolean
  unknown_field_policy: preserve | warn | error
  lossy_field_policy: preserve | warn | error
  backcompat_policy: warn | migrate | reject
  fixture_paths: [string]
  golden_examples: [string]
  adapter_self_test_suite: [string]
```

- [ADP-041] Every adapter **MUST** define a validation profile.
- [ADP-042] A canonical adapter release **MUST** ship at least one self-test suite and one golden fixture set.

## Section 3: NORMALIZED NATIVE SURFACES

### 3.1 Planning hierarchy
The adapter must handle at least these classes:
- strategy / initiative
- epic
- story
- slice
- task
- review
- evidence bundle
- handoff
- escalation or scope-change artifact
- decision filter / north-star artifact

- [ADP-043] The adapter **MUST** be able to ingest hierarchical planning systems.
- [ADP-044] The adapter **MAY** infer missing levels, but inferred levels **MUST** be marked as synthesized.

### 3.2 Execution and dispatch surfaces
The adapter must support:
- sequential DEV→QA pairs
- parallel windows
- wave batches
- fan-out / fan-in groups
- queue or workflow packets
- loop/revision cycles
- handoff artifacts

- [ADP-045] Dispatch metadata **MUST** be representable even when the native system stores it in prose rather than structured data.

### 3.3 Evidence and governance surfaces
The adapter must support:
- evidence bundles
- review reports
- handoffs
- scope change requests
- waivers
- escalation records
- calibration artifacts

- [ADP-046] Evidence and governance artifacts **MUST** be mappable independently of planning artifacts.

## Section 4: RECURSIVE-PLANNING BRIDGE FAMILY

### 4.1 plan_revision_event
A revision-stream event.

```yaml
plan_revision_event:
  artifact_meta: {}
  plan_id: string
  revision_id: string
  revision_index: integer
  parent_revision_id: optional string
  trigger: string
  changed_artifacts: [string]
  changed_assumptions: [string]
  changed_dependencies: [string]
  changed_boundaries: [string]
  revision_reason: string
```

### 4.2 loop_state
A compact planning ledger.

```yaml
loop_state:
  artifact_meta: {}
  plan_id: string
  current_goal: string
  current_subtask: string
  progress_assessment: string
  replan_needed: boolean
  blocked_by: [string]
  open_questions: [string]
  completion_check: string
  iteration_count: integer
  max_iterations: integer
```

### 4.3 execution_target
The normalized target about to execute.

```yaml
execution_target:
  artifact_meta: {}
  target_type: story | slice | task | work_order | packet | custom
  target_id: string
  target_scope: string
  side_effect_class: none | read_only | reversible | irreversible
  execution_safety_mode: sandbox | staging | production_read_only | production_write
  required_approvals: [string]
  preconditions: [string]
```

### 4.4 feedback_artifact
Planner-facing Holdout feedback.

```yaml
feedback_artifact:
  artifact_meta: {}
  feedback_id: string
  source_finding_ids: [string]
  source_observation_ids: [string]
  requirement_refs: [string]
  must_preserve: [string]
  new_risks: [string]
  missing_context: [string]
  missing_escalation: [string]
  boundary_legibility_gaps: [string]
  plan_delta_required: string
  next_revision_expectation: string
```

### 4.5 recursive_evaluation_hook
The hook policy object.

```yaml
recursive_evaluation_hook:
  artifact_meta: {}
  hook_id: string
  hookpoint: pre_decomposition | post_decomposition | pre_execution | post_execution | side_effect_boundary | fanout_boundary | convergence_boundary
  authority_level: advise | recommend | block
  trigger_condition: string
  budget_limit: string
  block_on_fail: boolean
  emit_artifacts: [string]
  escalation_path: string
```

### 4.6 plan_convergence_gate
The recursive stop/go gate.

```yaml
plan_convergence_gate:
  artifact_meta: {}
  gate_id: string
  batch_id: string
  revision_window: string
  blocker_delta: integer
  mdr: number
  feedback_uptake_latency: optional number
  stale_subplan_incidence: optional number
  convergence_disagreement: optional number
  praise_survival: optional number
  verdict: pass | fail | pass_with_waivers | incomplete
```

- [ADP-047] The adapter **MAY** support recursive bridge schemas only when they are declared in `recursive_bridge_schemas`.
- [ADP-048] The adapter **MUST NOT** imply that these schemas are part of the core Holdout registry unless a later promotion process says so.

## Section 5: DEFAULT HOOK POLICY

The canonical default is **revision-boundary-safe**:

Trigger Holdout recursively only:
1. after a typed plan revision,
2. before irreversible side effects,
3. at fan-out/fan-in boundaries,
4. at convergence decisions,
5. and when a planner explicitly requests advisory evaluation.

Do not default to every thought, every task decomposition, or every memory write.

- [ADP-049] The default hook profile **MUST** be `revision_boundary_safe`.
- [ADP-050] `full_cycle_experimental` **MUST** be opt-in and marked experimental.

## Section 6: IMPLEMENTATION PATTERNS

### 6.1 File-native adapter
For markdown/yaml repo systems.
Maps:
- strategy / epic / story / slice files into `evaluation_batch` context and scenario seeds
- handoffs, evidence bundles, review reports, and DAG files into provenance, evidence, and dispatch surfaces
- `builder_packet` into rendered markdown work orders

### 6.2 DB-backed adapter
For SQLite/Postgres/API-native systems.
Maps:
- row sets into canonical artifacts,
- native foreign keys into `artifact_id` / `trace_id` bridge fields,
- rendered outputs into rows, views, or downstream packet emitters.

### 6.3 Generic open-source adapter
A minimal reference adapter that:
- assumes no specific PM vendor,
- supports file + JSONL + SQLite,
- ships safe example configs,
- and keeps all local assumptions in the manifest.

- [ADP-051] The public adapter package **SHOULD** ship at least one file-native example and one DB-backed example.

## Section 7: SECURITY, PUBLICATION, AND OPEN-SOURCE SAFETY

The adapter must be publishable without becoming a vulnerability amplifier.

Rules:
- no secrets in example configs
- no automatic execution of commands embedded in YAML or markdown
- safe YAML parsing only
- explicit path allowlists
- redaction before publication
- public release evidence separated from private runtime outputs
- rendered work orders treated as text artifacts, not auto-executable intent
- provenance captured, but PII and internal topology suppressed where publication would create risk

- [ADP-052] Example configs **MUST** be secret-free placeholders.
- [ADP-053] Public self-eval artifacts **MUST** be sanitized.
- [ADP-054] Private runtime traces **MUST NOT** be committed by default.

## Section 8: VALIDATION AND PROMOTION

An adapter family is canonical only when it can validate itself.

Minimum validation obligations:
1. schema validation against the adapter registry,
2. round-trip validation on golden fixtures,
3. one file-native self-test,
4. one DB or workflow self-test,
5. one publication-safety check,
6. one compatibility statement against Holdout Skill v2.1.1.

Promotion rule:
- no bridge schema enters the Holdout core until at least one graph runtime and one workflow runtime have validated it and Ring 3 review approves it.

- [ADP-055] A canonical adapter release **MUST** ship an adapter validation report.
- [ADP-056] Promotion of adapter-local bridge schemas into the core **MUST** remain blocked until multi-runtime validation plus Ring 3 review.

## Section 9: META-SPECIFICATION

This adapter protocol is intentionally leaner than the Holdout Skill core. It exists to prevent local orchestration variety from causing doctrinal churn. It should evolve conservatively.

Known limits:
- it does not yet prescribe one universal DB schema,
- it does not canonize recursive-planning taxonomy codes,
- it does not solve convergent-causation graph modeling,
- it does not solve in-region cultural validation,
- it does not yet provide official adapters for named runtimes,
- and it does not claim that every native system can round-trip without loss.

- [ADP-057] Adapter evolution **MUST** remain subordinate to the frozen Holdout core unless formal promotion occurs.
- [ADP-058] The adapter bundle's own release pipeline **MUST** run a YAML well-formedness gate over every example before freezing — the gate class that would have caught the v0.1.0 generic-example defect at source.

**Errata fold-in (v0.1.0 → v0.2.0).** The v0.2.0 examples fold in ERRATA-1 against the v0.1.0 generic example: `examples/generic_open_source_adapter.example.yaml` line 84 is quoted (`naming_convention: "{finding_id}.md"`) so the document parses under strict YAML. The corrected file is byte-identical to the errata-1 delivery (SHA256 `349ff369…`); the other two examples are byte-identical to v0.1.0. Provenance chain: v0.1.0 delivery → errata-1 delivery → v0.2.0 delivery. ADP-058 above is the permanent gate that makes this class of defect structurally unable to ship again.

## Section 10: FINAL FORMULATION

**The Holdout Adapter is the canonical translation boundary between a stable, doctrine-governed evaluation skill and unstable local orchestration systems. It keeps planning, dispatch, storage, rendering, provenance, recursive overlay, and publication policy explicit, typed, and replaceable so the Holdout Skill can remain one public core across many private or public runtimes.**
