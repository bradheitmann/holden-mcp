# The synthetic client's own dispatch-graph validator.
def topo($n): {done:[], left:$n, stuck:false}
  | until((.left|length)==0 or .stuck;
      .done as $d
      | [.left[] | select(([.depends_on[]?] - $d)|length==0) | .id] as $ready
      | if ($ready|length)==0 then .stuck=true
        else .done += $ready | .left |= map(select(.id as $x | ($ready|index($x))|not)) end);
.nodes as $n
| ($n|map(.id)) as $ids
| [$n[] | .depends_on[]? | select(. as $d | ($ids|index($d))|not)] as $missing
| [$n[] | select(.type=="QA") | .verifies | select(. as $v | ($ids|index($v))|not)] as $badv
| [$n[] | select(.type=="DEV") | .id as $d | select(([$n[] | select(.type=="QA" and .verifies==$d)]|length)!=1) | $d] as $unpaired
| [$n[] | select(.type=="QA") | select(.verifies as $v | ([.depends_on[]?]|index($v))|not) | .id] as $qanodep
| [$n[] | select(.type=="QA") | select(.fresh_context!=true) | .id] as $qanotfresh
| [$n[] | select(.type=="DEV") | select(.qa_contract_authored_before_implementation!=true) | .id] as $noqafirst
| [$n[] | select(.type=="DEV") | select((.contract|has("objective") and has("resource_scope") and has("acceptance_criterion") and has("budget") and has("target_selector"))|not) | .id] as $nocontract
| [$n[] | select(.type=="HOLDOUT") | select([.depends_on[]? as $d | $n[] | select(.id==$d) | .type] | all(.=="QA")|not) | .id] as $holdoutbad
| [$n[] | select(.type|IN("DEV","QA","HOLDOUT","OWNER")|not) | .id] as $badtype
| topo($n) as $t
| ([.dependency_frontiers[][]]|sort) as $fr
| { nodes: ($n|length), dev: ([$n[]|select(.type=="DEV")]|length), qa: ([$n[]|select(.type=="QA")]|length),
    holdout: ([$n[]|select(.type=="HOLDOUT")]|length), owner: ([$n[]|select(.type=="OWNER")]|length),
    pairs_declared: .dev_qa_pairs, missing_deps: $missing, qa_verifies_unknown: $badv,
    dev_without_exactly_one_qa: $unpaired, qa_not_depending_on_its_dev: $qanodep, qa_not_fresh: $qanotfresh,
    dev_without_qa_contract_first: $noqafirst, dev_without_full_contract: $nocontract,
    holdout_not_after_qa: $holdoutbad, unknown_types: $badtype,
    acyclic: ($t.stuck|not), topo_order_len: ($t.done|length),
    frontiers: (.dependency_frontiers|length), frontiers_cover_all_once: ($fr == ($ids|sort)),
    fast_lane_enabled: .fast_lane.enabled }
| . + {valid: (.missing_deps==[] and .qa_verifies_unknown==[] and .dev_without_exactly_one_qa==[] and .qa_not_depending_on_its_dev==[] and .qa_not_fresh==[] and .dev_without_qa_contract_first==[] and .dev_without_full_contract==[] and .holdout_not_after_qa==[] and .unknown_types==[] and .acyclic and .frontiers_cover_all_once and .dev==.pairs_declared and .qa==.pairs_declared)}
