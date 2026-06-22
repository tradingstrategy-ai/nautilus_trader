# Hyperliquid vault creation and agent wallets

This runbook records the verified paths for Hyperliquid vault creation and the
boundary between vault creation and approved agent wallets.

## Decision

Do not use an approved agent wallet to create a vault for a funded master or
multisig account.

Approved agent wallets are valid for post-creation trading and account actions
that Hyperliquid permits an agent to perform on behalf of the master. They are
not a way to spend the master's USDC on `createVault`.

Verified live result:

- signer: approved agent wallet
- approving account: funded master/multisig
- action: direct single-sig `createVault`
- response: `{"status":"err","response":"Insufficient balance to create vault"}`

This proves Hyperliquid checks the vault-creation balance against the signing
wallet that becomes the vault leader. A `master_address` in local config can be
correct metadata for trading, but it does not alter Hyperliquid's server-side
`createVault` balance check.

## Why this is not the same as `approveAgent`

Hyperliquid has two relevant signing paths:

| Action family | Example | Domain | Chain id |
| --- | --- | --- | --- |
| User-signed action | `approveAgent` | `HyperliquidSignTransaction` | `421614` |
| L1 action | `createVault` | `Exchange` | `1337` |

The `1337` value is part of the canonical Hyperliquid L1 EIP-712 signing
domain. It is not a wallet network that should be added to Rabby or MetaMask.

## Valid creation paths

### Direct funded EOA leader

Use a private key only when that EOA should become the vault leader and that
same EOA has enough Hyperliquid spot USDC for:

- 100 USDC vault creation fee.
- The initial vault deposit, minimum 100 USDC.

The strategy deployment repository contains the direct EOA script:

```text
scripts/hyperliquid-vault/create_vault.py
```

That script must reject approved-agent profiles for `createVault`; otherwise a
dry-run can look ready while the real exchange call fails on the signer's
balance.

### Multisig-held funds

When the funds belong to a Hyperliquid multisig, vault creation must use the
Hyperliquid `multiSig` envelope:

- `payload.multiSigUser` is the funded multisig address.
- The inner action is `createVault`.
- The inner signatures come from enough authorized multisig signers.
- The outer submission is signed by an authorized signer or by an API wallet of
  an authorized signer.

An approved trading agent for the multisig is not a substitute for the required
multisig approval on vault creation.

## Action body

The `createVault` action body has this shape:

```json
{
  "type": "createVault",
  "name": "Anti Ape AI",
  "description": "Systematic, AI-driven market-wide strategy that trades against crowded positioning across Hyperliquid perps. Powered by TradingStrategy.ai",
  "initialUsd": 100000000,
  "nonce": 1782118047282
}
```

Field notes:

- `name` is immutable after vault creation.
- `description` is immutable after vault creation.
- `initialUsd` is an integer in USDC base units. `100000000` means `100 USDC`.
- `nonce` is milliseconds since epoch.

## L1 signature construction

For direct L1 signing, the signer constructs the Hyperliquid L1 signature as
follows:

1. MessagePack-encode the `action` object.
2. Append the 8-byte big-endian `nonce`.
3. Append the vault-address marker:
   - `0x00` for account-level actions with no vault address.
   - `0x01 + 20-byte address` when routing to a vault/sub-account.
4. Keccak-hash the resulting bytes to produce `connectionId`.
5. Sign EIP-712 typed data:

```json
{
  "domain": {
    "name": "Exchange",
    "version": "1",
    "chainId": 1337,
    "verifyingContract": "0x0000000000000000000000000000000000000000"
  },
  "types": {
    "Agent": [
      { "name": "source", "type": "string" },
      { "name": "connectionId", "type": "bytes32" }
    ]
  },
  "primaryType": "Agent",
  "message": {
    "source": "a",
    "connectionId": "0x..."
  }
}
```

`source` is `"a"` for mainnet and `"b"` for testnet.

The direct single-sig exchange request sent to `/exchange` is:

```json
{
  "action": {
    "type": "createVault",
    "name": "...",
    "description": "...",
    "initialUsd": 100000000,
    "nonce": 1782118047282
  },
  "nonce": 1782118047282,
  "signature": {
    "r": "0x...",
    "s": "0x...",
    "v": 27
  }
}
```

For multisig-held funds, this direct request must be wrapped in the Hyperliquid
`multiSig` envelope described above.

## What `noop` proves

`noop` proves a key can sign the Hyperliquid L1 domain and that Hyperliquid
accepts the signature path. It does not prove `createVault` will pass the
vault-leader balance check.

Do not treat a successful `noop` as proof that an unfunded approved agent can
create a vault for a funded master.

## Known browser-wallet failure mode

The following browser wallet attempts were tested and did not produce a usable
L1 signature in the Rabby/Ledger setup:

- `eth_signTypedData_v4`: `chainId should be same as current chainId`
- `eth_signTypedData_v3`: `chainId should be same as current chainId`
- `eth_sign`: blocked by Rabby
- unversioned typed-data variants: unsupported parameter shapes

Do not repeat those fallback experiments unless the wallet/provider stack
changes.

## Safety checklist

Before creating a vault:

- Confirm whether the intended vault leader is a funded EOA or a funded
  Hyperliquid multisig.
- Do not use an approved agent-of-master key for `createVault`.
- Confirm `name`, `description`, and `initialUsd` are final.
- Confirm the signer or multisig account that will become the leader has enough
  available spot USDC.
- Run only no-cost preflights that test the same signing and account path you
  will use for the final submission.
