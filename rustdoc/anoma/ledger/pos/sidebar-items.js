initSidebarItems({"constant":[["ADDRESS","Address of the PoS account implemented as a native VP"],["SLASH_POOL_ADDRESS","Address of the PoS slash pool account"]],"fn":[["bond_key","Storage key for a bond with the given ID (source and validator)."],["bonds_for_source_prefix","Storage key prefix for all bonds of the given source address."],["bonds_prefix","Storage key prefix for all bonds."],["init_genesis_storage","Initialize storage in the genesis block."],["is_bond_key","Is storage key for a bond?"],["is_params_key","Is storage key for PoS parameters?"],["is_pos_key","Is the given key a PoS storage key?"],["is_total_voting_power_key","Is storage key for total voting power?"],["is_unbond_key","Is storage key for a unbond?"],["is_validator_consensus_key_key","Is storage key for validator’s consensus key?"],["is_validator_set_key","Is storage key for a validator set?"],["is_validator_slashes_key","Is storage key for validator’s slashes?"],["is_validator_staking_reward_address_key","Is storage key for validator’s staking reward address?"],["is_validator_state_key","Is storage key for validator’s state?"],["is_validator_total_deltas_key","Is storage key for validator’s total deltas?"],["is_validator_voting_power_key","Is storage key for validator’s voting power?"],["params_key","Storage key for PoS parameters."],["slashes_prefix","Storage prefix for slashes."],["staking_token_address","Address of the staking token (XAN)"],["total_voting_power_key","Storage key for total voting power."],["unbond_key","Storage key for an unbond with the given ID (source and validator)."],["unbonds_for_source_prefix","Storage key prefix for all unbonds of the given source address."],["unbonds_prefix","Storage key prefix for all unbonds."],["validator_address_raw_hash_key","Storage key for validator’s address raw hash for look-up from raw hash of an address to address."],["validator_consensus_key_key","Storage key for validator’s consensus key."],["validator_set_key","Storage key for validator set (active and inactive)."],["validator_slashes_key","Storage key for validator’s slashes."],["validator_staking_reward_address_key","Storage key for validator’s staking reward address."],["validator_state_key","Storage key for validator’s state."],["validator_total_deltas_key","Storage key for validator’s total deltas."],["validator_voting_power_key","Storage key for validator’s voting power."]],"mod":[["types","Proof of Stake data types"],["vp","Proof-of-Stake native validity predicate."]],"struct":[["PosParams","Proof-of-Stake system parameters"],["Slash","A slash applied to validator, to punish byzantine behavior by removing their staked tokens at and before the epoch of the slash."]],"type":[["BondId","Alias for a PoS type with the same name with concrete type parameters"],["Bonds","Alias for a PoS type with the same name with concrete type parameters"],["GenesisValidator","Alias for a PoS type with the same name with concrete type parameters"],["Slashes","Slashes applied to validator, to punish byzantine behavior by removing their staked tokens at and before the epoch of the slash."],["TotalVotingPowers","Epoched total voting power."],["Unbonds","Alias for a PoS type with the same name with concrete type parameters"],["ValidatorConsensusKeys","Alias for a PoS type with the same name with concrete type parameters"],["ValidatorSets","Alias for a PoS type with the same name with concrete type parameters"],["ValidatorStates","Epoched validator’s state."],["ValidatorTotalDeltas","Alias for a PoS type with the same name with concrete type parameters"],["ValidatorVotingPowers","Epoched validator’s voting power."]]});