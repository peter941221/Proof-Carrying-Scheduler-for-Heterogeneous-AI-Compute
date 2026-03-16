---- MODULE EvidenceChain ----
EXTENDS Naturals, Sequences

CONSTANTS DecisionIds, SnapshotHashes, Hashes

VARIABLES log

NoPrev == "NO_PREV"

Bundle ==
  [ decision_id: DecisionIds,
    snapshot_hash: SnapshotHashes,
    bundle_hash: Hashes,
    prev_bundle_hash: Hashes \cup {NoPrev} ]

LogSet == { log[i] : i \in 1..Len(log) }

DecisionIdsInLog == { b.decision_id : b \in LogSet }

TypeOK ==
  /\ log \in Seq(Bundle)

WellFormedAppend(b) ==
  /\ b \in Bundle
  /\ b.decision_id \notin DecisionIdsInLog
  /\ IF Len(log) = 0
        THEN b.prev_bundle_hash = NoPrev
        ELSE b.prev_bundle_hash = log[Len(log)].bundle_hash

Init ==
  /\ log = << >>

Append ==
  \E b \in Bundle:
    /\ WellFormedAppend(b)
    /\ log' = Append(log, b)

Next ==
  Append

Inv_UniqueDecisionIds ==
  \A i, j \in 1..Len(log):
    (i # j) => (log[i].decision_id # log[j].decision_id)

Inv_ChainLink ==
  /\ IF Len(log) = 0 THEN TRUE ELSE log[1].prev_bundle_hash = NoPrev
  /\ \A i \in 2..Len(log):
       log[i].prev_bundle_hash = log[i - 1].bundle_hash

Inv_SnapshotBinding ==
  \A i \in 1..Len(log):
    log[i].snapshot_hash \in SnapshotHashes

Inv_AppendOnlyShape ==
  Len(log') >= Len(log)

AppendProgress ==
  <>(Len(log) > 0)

Spec ==
  Init /\ [][Next]_log

====
