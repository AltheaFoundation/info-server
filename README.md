# Althea L1 Info Dashboard

This repo contains frontend and backend code for deploying a Althea L1 information server, this server queries public full nodes and processes data about the chain in order to present the page https://info.althea.zone in addition to this page public API endpoints are provided

This server computes and displays info that would be otherwise difficult to access or compute. Such as token supply metrics

TODO: 
MicroTx volume
EVM transaction fee volume

Issues and pull requests for new endpoints or information formats are welcome.

The repo auto-deploys the most recent commit in main to https://info.althea.zone

## API Docs

### /total_supply

Provides the total supply of ALTHEA, or any Cosmos chain the server software is pointed at. This is inclusive of the community pool, vesting tokens, staked tokens, and unclaimed rewards. Value return is aalthea (ALTHEA wei) and must be divided by `1*10^18` to display whole tokens. This value is updated once a day.

- URL: `https://info.althea.zone:9000/total_supply`
- Method: `GET`
- URL Params: `None`
- Data Params: `None`
- Success Response:
  - Code: 200 OK
  - Contents:

```
"423746179291553"
```

- Error Response: `500 Server Error`

- Sample Call:

`curl https://info.althea.zone:9000/total_supply`

---

### /total_liquid_supply

Provides the total liquid supply of ALTHEA, or any Cosmos chain the server software is pointed at. Liquid supply excludes only module tokens and vesting tokens. Staked tokens and unclaimed rewards count in the total. Value return is aalthea (ALTHEA wei) and must be divided by `1*10^18` to display whole tokens. This value is updated once a day.

- URL: `https://info.althea.zone:9000/total_liquid_supply`
- Method: `GET`
- URL Params: `None`
- Data Params: `None`
- Success Response:
  - Code: 200 OK
  - Contents:

```
"423746179291553"
```

- Error Response: `500 Server Error`

- Sample Call:

`curl https://info.althea.zone:9000/total_liquid_supply`

---

### /supply_info

Provides a breakdown of vesting versus non-vesting tokens for ALTHEA, value returned are in aalthea (ALTHEA wei) and must be divided by `1*10^18` to display whole tokens. This value is updated once a day.

* total_supply: The total supply of tokens in existance.
* community_pool: The total amount of tokens in the community pool subject to use by governance vote
* total_liquid_supply: All tokens that are not vesting and not in the community pool, this includes staked tokens and unclaimed staking rewards.
* total_liquid_balances: Tokens that are avaialble to be sent immeidately, so tokens that are not staked and not vesting.
* total_nonvesting_staked: These tokens are liquid (eg not vesting) and currently staked.
* total_vesting: A sum of all tokens that are not yet vested but will become liquid at some point in the future.
* total_vesting_staked: All tokens that are vesting and also staked
* total_vested: The amount of tokens that where once vesting but are now liquid

- URL: `https://info.althea.zone:9000/supply_info`
- Method: `GET`
- URL Params: `None`
- Data Params: `None`
- Success Response:
  - Code: 200 OK
  - Contents:

```
{
  "total_supply": "2489386289699730",
  "community_pool": "938460578037767",
  "total_liquid_supply": "475122384773913",
  "total_liquid_balances": "151777718973370",
  "total_unclaimed_rewards": "107181985809999",
  "total_nonvesting_staked": "192953527166768",
  "total_vesting": "1050344613544263",
  "total_vesting_staked": "897039356148458",
  "total_vested": "22484483020980"
}


```

- Error Response: `500 Server Error`

- Sample Call:

`curl https://info.althea.zone:9000/supply_info`

---
