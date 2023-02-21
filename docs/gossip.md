# Gossip Service

## CRDS (Cluster Replicated Data Structure)
The CRDS inspired by Solana's CRDS is a data structure that holds information about each node replicated across each node in a leaderless mechanism.

```txt
CRDS
├── Version // Version of the nodes copy
├── CrdsValueLabel(Pubkey1) -> CRDSValue
├── CrdsValueLabel(Pubkey2) -> CRDSValue
├── CrdsValueLabel(Pubkey3) -> CRDSValue

CRDSValue
├── Header -> CRDSValueHeader
├── Data -> CRDSData


CRDSValueHeader
├── Pubkey
├── Signature

CRDSData // Enum that can be one of the following
├── archived_shreds -> ArchivedShreds // Archived shreds of the node

├── contact_info -> ContactInfo // contact info of the node including ports and ip
--------------------------------

ArchivedShreds
├── Slot1 -> (u64,ShredType)[] // shred index and type
├── Slot2 -> (u64,ShredType)[]
├── Slot3 -> (u64,ShredType)[]
```

The structure stores a hashmap of pubkeys and their associeated CRDSValues. The values have a header which has the nodes pubkey and signature, these two are used to verify when a write request is made if 1) the node is who it says it is and 2) the node is party of the network and allowed to make write requests. THe values also contain a hashmap of slots and their respective shreds.

Each Node would create the CRDS value to be appended, append their local CRDS structure in db and then broadcast the change to all nodes in the CRDS, based on the quorum set by each node it accepts confirmation from the respective nodes about the change. This process is synchronous which makes sure there is always a copy of the CRDS on a certain number of nodes.

The node making the write will send the change to all other nodes in parallel.

When reading CRDS from the network, you request from multiple nodes and accept the structure with the highest version. This is done to ensure a stale copy isn't returned.

Each Node also has an anti entropy process running the background to update any stale copy it has of the CRDS.

If n is the number of nodes, w is the quorum for confirmation of replication and r is the number of nodes to send a read request then w+r > n to ensure a stale copy isn't accepted.

We also need to ensure that w and r are always a majority of n i.e w or r > n/2 because
r needs to overlap with w to get the latest copy.

### Concurrent Writes

Two Nodes can't write to the same key because each Node can only write to their key and each node is unique in the network. This means the only case here would be Node A and Node B writing to the CRDS with keys that don't overlap i.e both the nodes don't have information about each others writes at the moment. This isn't an issue for our use case and since we are using a hashmap we don't have to worry about overriding values unintentionally unlike in an array. We don't even have to worry about merging changes because we only stream the changes operations to the other nodes.