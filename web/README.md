# Cinder web

The Cinder product site lives here. It has two static routes:

- `/` explains Cinder as a private perpetual-futures prime broker for Phoenix.
- `/terminal` combines live, read-only Phoenix candles and order-book data
  with a deterministic walkthrough of Cinder private intent, acknowledgement,
  and netting.

The terminal is intentionally non-custodial and non-executing: it does not
connect wallets, collect credentials, or route orders.

## Run locally

```bash
yarn install
yarn dev
```

Run `yarn build` before a release. Brand tokens and voice are recorded in the
repository-root `brand.md`.

## Deploy to Vercel

Deploy `web/` as a standalone Vercel project; do not deploy the repository
root, which is the Anchor workspace.

1. Import the GitHub repository in Vercel.
2. Set **Root Directory** to `web`.
3. Keep the detected **Next.js** framework and the default build settings.
4. Optionally add `PHOENIX_API_URL=https://perp-api.phoenix.trade` to the
   Production, Preview, and Development environment targets. It is public and
   has the same value as the built-in default.
5. Deploy. The `/api/market` route fetches public Phoenix data server-side and
   uses a two-second CDN cache with an eight-second stale-while-revalidate
   window to avoid multiplying upstream requests.

After deployment, visit `/terminal` and verify that the status moves from
`LOADING` to `POLLING` or `LIVE`. `POLLING` is still real public Phoenix data;
it is the fallback when the browser WebSocket cannot connect.
