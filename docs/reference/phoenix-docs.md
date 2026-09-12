# Aggregated Documentation

Total files: 114

---

# Source: `api/auth/create-service-login-challenge.md`

**Original URL:** `https://docs.phoenix.trade/api/auth/create-service-login-challenge.md`

**Local path:** `api/auth/create-service-login-challenge.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Create service login challenge

> Handles `POST /v1/auth/login/service/challenge` via `post.v1.auth.login.service.challenge`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json post /v1/auth/login/service/challenge
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/auth/login/service/challenge:
    post:
      tags:
        - Auth
      summary: Create service login challenge
      description: >-
        Handles `POST /v1/auth/login/service/challenge` via
        `post.v1.auth.login.service.challenge`.
      operationId: post.v1.auth.login.service.challenge
      requestBody:
        description: JSON request payload for `post.v1.auth.login.service.challenge`.
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/ServiceChallengeRequest'
            example:
              client_id: string
        required: true
      responses:
        '200':
          description: Challenge issued
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ServiceChallengeResponse'
        '400':
          $ref: '#/components/responses/ErrorResponse'
        '404':
          $ref: '#/components/responses/ErrorResponse'
        '500':
          $ref: '#/components/responses/ErrorResponse'
components:
  schemas:
    ServiceChallengeRequest:
      type: object
      required:
        - client_id
      properties:
        client_id:
          type: string
        key_id:
          type:
            - string
            - 'null'
    ServiceChallengeResponse:
      type: object
      required:
        - nonce
        - message
        - expires_at
        - key_id
      properties:
        expires_at:
          type: string
        key_id:
          type: string
        message:
          type: string
        nonce:
          type: string
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'

````

<!-- END SOURCE: api/auth/create-service-login-challenge.md -->

---

# Source: `api/auth/create-wallet-transaction-challenge.md`

**Original URL:** `https://docs.phoenix.trade/api/auth/create-wallet-transaction-challenge.md`

**Local path:** `api/auth/create-wallet-transaction-challenge.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Create wallet transaction challenge

> Handles `POST /v1/auth/wallet/transaction-challenge` via `post.v1.auth.wallet.transaction_challenge`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json post /v1/auth/wallet/transaction-challenge
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/auth/wallet/transaction-challenge:
    post:
      tags:
        - Auth
      summary: Create wallet transaction challenge
      description: >-
        Handles `POST /v1/auth/wallet/transaction-challenge` via
        `post.v1.auth.wallet.transaction_challenge`.
      operationId: post.v1.auth.wallet.transaction_challenge
      requestBody:
        description: JSON request payload for `post.v1.auth.wallet.transaction_challenge`.
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/WalletTransactionChallengeRequest'
            example:
              wallet_pubkey: string
        required: true
      responses:
        '200':
          description: Challenge issued
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/WalletTransactionChallengeResponse'
        '400':
          $ref: '#/components/responses/ErrorResponse'
        '500':
          $ref: '#/components/responses/ErrorResponse'
        '503':
          $ref: '#/components/responses/ErrorResponse'
components:
  schemas:
    WalletTransactionChallengeRequest:
      type: object
      required:
        - wallet_pubkey
      properties:
        wallet_pubkey:
          type: string
          description: |-
            Solana wallet pubkey the caller intends to authenticate as. The
            challenge transaction is built with this pubkey as the fee payer /
            sole signer.
    WalletTransactionChallengeResponse:
      type: object
      required:
        - nonce_id
        - unsigned_transaction
        - expires_at
      properties:
        expires_at:
          type: string
          description: RFC3339 timestamp at which this challenge expires.
        nonce_id:
          type: string
          description: |-
            Opaque server-issued nonce identifier. Echo on the subsequent login
            call.
        unsigned_transaction:
          type: string
          description: >-
            Base64-encoded unsigned Solana legacy transaction (bincode wire

            format) containing a single Memo program instruction that encodes
            the

            login challenge. Wallets are allowed to include additional

            ComputeBudget or Lighthouse instructions before signing (some inject

            these automatically, in arbitrary positions), but the Memo

            instruction, the recent blockhash, and the wallet's required-signer

            slot must be preserved. The transaction uses a deterministic,

            non-recent blockhash and is never submitted on-chain.
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'

````

<!-- END SOURCE: api/auth/create-wallet-transaction-challenge.md -->

---

# Source: `api/auth/get-jwks.md`

**Original URL:** `https://docs.phoenix.trade/api/auth/get-jwks.md`

**Local path:** `api/auth/get-jwks.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get JWKS

> Handles `GET /v1/auth/jwks` via `get.v1.auth.jwks`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/auth/jwks
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/auth/jwks:
    get:
      tags:
        - Auth
      summary: Get JWKS
      description: Handles `GET /v1/auth/jwks` via `get.v1.auth.jwks`.
      operationId: get.v1.auth.jwks
      responses:
        '200':
          description: JWKS returned
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/JwksResponse'
        '500':
          $ref: '#/components/responses/ErrorResponse'
components:
  schemas:
    JwksResponse:
      type: object
      required:
        - keys
      properties:
        keys:
          type: array
          items: {}
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'

````

<!-- END SOURCE: api/auth/get-jwks.md -->

---

# Source: `api/auth/get-wallet-login-nonce.md`

**Original URL:** `https://docs.phoenix.trade/api/auth/get-wallet-login-nonce.md`

**Local path:** `api/auth/get-wallet-login-nonce.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get wallet login nonce

> Handles `GET /v1/auth/nonce` via `get.v1.auth.nonce`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/auth/nonce
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/auth/nonce:
    get:
      tags:
        - Auth
      summary: Get wallet login nonce
      description: Handles `GET /v1/auth/nonce` via `get.v1.auth.nonce`.
      operationId: get.v1.auth.nonce
      parameters:
        - name: wallet_pubkey
          in: query
          required: true
          schema:
            type: string
      responses:
        '200':
          description: Nonce issued
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/WalletNonceResponse'
        '400':
          $ref: '#/components/responses/ErrorResponse'
        '500':
          $ref: '#/components/responses/ErrorResponse'
components:
  schemas:
    WalletNonceResponse:
      type: object
      required:
        - nonce_id
        - message
        - expires_at
      properties:
        expires_at:
          type: string
        message:
          type: string
        nonce_id:
          type: string
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'

````

<!-- END SOURCE: api/auth/get-wallet-login-nonce.md -->

---

# Source: `api/auth/log-in-with-privy-token.md`

**Original URL:** `https://docs.phoenix.trade/api/auth/log-in-with-privy-token.md`

**Local path:** `api/auth/log-in-with-privy-token.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Log in with Privy token

> Handles `POST /v1/auth/login/privy` via `post.v1.auth.login.privy`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json post /v1/auth/login/privy
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/auth/login/privy:
    post:
      tags:
        - Auth
      summary: Log in with Privy token
      description: Handles `POST /v1/auth/login/privy` via `post.v1.auth.login.privy`.
      operationId: post.v1.auth.login.privy
      requestBody:
        description: JSON request payload for `post.v1.auth.login.privy`.
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/PrivyLoginRequest'
            example:
              privy_token: string
        required: true
      responses:
        '200':
          description: Phoenix JWT pair issued
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/AuthResponse'
        '400':
          $ref: '#/components/responses/ErrorResponse'
        '401':
          $ref: '#/components/responses/ErrorResponse'
        '403':
          $ref: '#/components/responses/ErrorResponse'
        '409':
          $ref: '#/components/responses/ErrorResponse'
        '500':
          $ref: '#/components/responses/ErrorResponse'
        '502':
          $ref: '#/components/responses/ErrorResponse'
        '503':
          $ref: '#/components/responses/ErrorResponse'
components:
  schemas:
    PrivyLoginRequest:
      type: object
      required:
        - privy_token
      properties:
        privy_token:
          type: string
        wallet_proof:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/PrivyWalletProof'
              description: >-
                Proof of ownership of `wallet_pubkey`, obtained by signing the

                transaction returned from `POST
                /v1/auth/privy/wallet-challenge`. When

                supplied this satisfies the first-time binding ownership check

                without needing the Privy linked-accounts lookup.
        wallet_pubkey:
          type:
            - string
            - 'null'
          description: >-
            Optional Solana wallet pubkey the caller is asserting ownership of.

            For first-time bindings ownership must be proved either by including

            `wallet_proof` (a Solana memo transaction signed by the wallet —
            works

            for Ledger which cannot sign off-chain messages) or, absent a proof,

            by Privy's `linked_accounts` API confirming the wallet is linked to

            the authenticated user. An unverified pubkey is rejected with

            `wallet_not_linked`. For returning users, if supplied, it must match

            the bound pubkey. If omitted when no mapping exists, login still

            succeeds but the issued token will not carry a `wallet` claim and so

            cannot satisfy user-scoped authorization.
    AuthResponse:
      type: object
      required:
        - token_type
        - access_token
        - expires_in
        - refresh_token
        - refresh_expires_in
        - pop_key
      properties:
        access_token:
          type: string
        expires_in:
          type: integer
          format: int64
          minimum: 0
        pop_key:
          type: string
        refresh_expires_in:
          type: integer
          format: int64
          minimum: 0
        refresh_token:
          type: string
        token_type:
          type: string
    PrivyWalletProof:
      type: object
      required:
        - nonce_id
        - signed_transaction
      properties:
        nonce_id:
          type: string
          description: The `nonce_id` returned from the wallet-challenge endpoint.
        signed_transaction:
          type: string
          description: |-
            The full signed Solana transaction, base64-encoded (legacy bincode
            format). The transaction must be the same one returned from the
            wallet-challenge endpoint, signed by `wallet_pubkey`.
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'

````

<!-- END SOURCE: api/auth/log-in-with-privy-token.md -->

---

# Source: `api/auth/log-in-with-service-signature.md`

**Original URL:** `https://docs.phoenix.trade/api/auth/log-in-with-service-signature.md`

**Local path:** `api/auth/log-in-with-service-signature.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Log in with service signature

> Handles `POST /v1/auth/login/service` via `post.v1.auth.login.service`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json post /v1/auth/login/service
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/auth/login/service:
    post:
      tags:
        - Auth
      summary: Log in with service signature
      description: Handles `POST /v1/auth/login/service` via `post.v1.auth.login.service`.
      operationId: post.v1.auth.login.service
      requestBody:
        description: JSON request payload for `post.v1.auth.login.service`.
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/ServiceLoginRequest'
            example:
              client_id: string
              nonce: string
              signature: string
              timestamp: string
        required: true
      responses:
        '200':
          description: Phoenix JWT pair issued
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/AuthResponse'
        '400':
          $ref: '#/components/responses/ErrorResponse'
        '401':
          $ref: '#/components/responses/ErrorResponse'
        '404':
          $ref: '#/components/responses/ErrorResponse'
        '409':
          $ref: '#/components/responses/ErrorResponse'
        '500':
          $ref: '#/components/responses/ErrorResponse'
components:
  schemas:
    ServiceLoginRequest:
      type: object
      required:
        - client_id
        - nonce
        - timestamp
        - signature
      properties:
        client_id:
          type: string
        key_id:
          type:
            - string
            - 'null'
        nonce:
          type: string
        signature:
          type: string
        timestamp:
          type: string
    AuthResponse:
      type: object
      required:
        - token_type
        - access_token
        - expires_in
        - refresh_token
        - refresh_expires_in
        - pop_key
      properties:
        access_token:
          type: string
        expires_in:
          type: integer
          format: int64
          minimum: 0
        pop_key:
          type: string
        refresh_expires_in:
          type: integer
          format: int64
          minimum: 0
        refresh_token:
          type: string
        token_type:
          type: string
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'

````

<!-- END SOURCE: api/auth/log-in-with-service-signature.md -->

---

# Source: `api/auth/log-in-with-signed-wallet-transaction.md`

**Original URL:** `https://docs.phoenix.trade/api/auth/log-in-with-signed-wallet-transaction.md`

**Local path:** `api/auth/log-in-with-signed-wallet-transaction.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Log in with signed wallet transaction

> Handles `POST /v1/auth/login/wallet/transaction` via `post.v1.auth.login.wallet.transaction`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json post /v1/auth/login/wallet/transaction
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/auth/login/wallet/transaction:
    post:
      tags:
        - Auth
      summary: Log in with signed wallet transaction
      description: >-
        Handles `POST /v1/auth/login/wallet/transaction` via
        `post.v1.auth.login.wallet.transaction`.
      operationId: post.v1.auth.login.wallet.transaction
      requestBody:
        description: JSON request payload for `post.v1.auth.login.wallet.transaction`.
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/WalletTransactionLoginRequest'
            example:
              nonce_id: string
              signed_transaction: string
              wallet_pubkey: string
        required: true
      responses:
        '200':
          description: Phoenix JWT pair issued
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/AuthResponse'
        '400':
          $ref: '#/components/responses/ErrorResponse'
        '403':
          $ref: '#/components/responses/ErrorResponse'
        '409':
          $ref: '#/components/responses/ErrorResponse'
        '500':
          $ref: '#/components/responses/ErrorResponse'
        '503':
          $ref: '#/components/responses/ErrorResponse'
components:
  schemas:
    WalletTransactionLoginRequest:
      type: object
      required:
        - wallet_pubkey
        - nonce_id
        - signed_transaction
      properties:
        nonce_id:
          type: string
          description: Echo of the `nonce_id` returned from the challenge endpoint.
        signed_transaction:
          type: string
          description: >-
            Base64-encoded fully-signed Solana legacy transaction (bincode wire

            format). Must be the challenge transaction signed by
            `wallet_pubkey`.

            Wallets are permitted to include additional ComputeBudget or

            Lighthouse instructions before signing (in any position); any other

            instruction (other than the Memo issued by the server) is rejected.

            The server verifies the signature, the memo payload, and that the

            message still uses the issued blockhash so the signed bytes are

            never broadcastable on-chain.
        wallet_pubkey:
          type: string
    AuthResponse:
      type: object
      required:
        - token_type
        - access_token
        - expires_in
        - refresh_token
        - refresh_expires_in
        - pop_key
      properties:
        access_token:
          type: string
        expires_in:
          type: integer
          format: int64
          minimum: 0
        pop_key:
          type: string
        refresh_expires_in:
          type: integer
          format: int64
          minimum: 0
        refresh_token:
          type: string
        token_type:
          type: string
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'

````

<!-- END SOURCE: api/auth/log-in-with-signed-wallet-transaction.md -->

---

# Source: `api/auth/log-in-with-wallet-signature.md`

**Original URL:** `https://docs.phoenix.trade/api/auth/log-in-with-wallet-signature.md`

**Local path:** `api/auth/log-in-with-wallet-signature.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Log in with wallet signature

> Handles `POST /v1/auth/login/wallet` via `post.v1.auth.login.wallet`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json post /v1/auth/login/wallet
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/auth/login/wallet:
    post:
      tags:
        - Auth
      summary: Log in with wallet signature
      description: Handles `POST /v1/auth/login/wallet` via `post.v1.auth.login.wallet`.
      operationId: post.v1.auth.login.wallet
      requestBody:
        description: JSON request payload for `post.v1.auth.login.wallet`.
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/WalletLoginRequest'
            example:
              nonce_id: string
              signature: string
              wallet_pubkey: string
        required: true
      responses:
        '200':
          description: Phoenix JWT pair issued
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/AuthResponse'
        '400':
          $ref: '#/components/responses/ErrorResponse'
        '401':
          $ref: '#/components/responses/ErrorResponse'
        '409':
          $ref: '#/components/responses/ErrorResponse'
        '500':
          $ref: '#/components/responses/ErrorResponse'
components:
  schemas:
    WalletLoginRequest:
      type: object
      required:
        - wallet_pubkey
        - signature
        - nonce_id
      properties:
        nonce_id:
          type: string
        signature:
          type: string
        wallet_pubkey:
          type: string
    AuthResponse:
      type: object
      required:
        - token_type
        - access_token
        - expires_in
        - refresh_token
        - refresh_expires_in
        - pop_key
      properties:
        access_token:
          type: string
        expires_in:
          type: integer
          format: int64
          minimum: 0
        pop_key:
          type: string
        refresh_expires_in:
          type: integer
          format: int64
          minimum: 0
        refresh_token:
          type: string
        token_type:
          type: string
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'

````

<!-- END SOURCE: api/auth/log-in-with-wallet-signature.md -->

---

# Source: `api/auth/log-out.md`

**Original URL:** `https://docs.phoenix.trade/api/auth/log-out.md`

**Local path:** `api/auth/log-out.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Log out

> Handles `POST /v1/auth/logout` via `post.v1.auth.logout`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json post /v1/auth/logout
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/auth/logout:
    post:
      tags:
        - Auth
      summary: Log out
      description: Handles `POST /v1/auth/logout` via `post.v1.auth.logout`.
      operationId: post.v1.auth.logout
      responses:
        '204':
          description: Session revoked
        '401':
          $ref: '#/components/responses/ErrorResponse'
        '404':
          $ref: '#/components/responses/ErrorResponse'
        '500':
          $ref: '#/components/responses/ErrorResponse'
      security:
        - PhoenixBearerAuth: []
components:
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'
  schemas:
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/auth/log-out.md -->

---

# Source: `api/auth/refresh-session.md`

**Original URL:** `https://docs.phoenix.trade/api/auth/refresh-session.md`

**Local path:** `api/auth/refresh-session.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Refresh session

> Handles `POST /v1/auth/refresh` via `post.v1.auth.refresh`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json post /v1/auth/refresh
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/auth/refresh:
    post:
      tags:
        - Auth
      summary: Refresh session
      description: Handles `POST /v1/auth/refresh` via `post.v1.auth.refresh`.
      operationId: post.v1.auth.refresh
      requestBody:
        description: JSON request payload for `post.v1.auth.refresh`.
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/RefreshRequest'
            example:
              refresh_token: string
        required: true
      responses:
        '200':
          description: New token pair issued
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/AuthResponse'
        '401':
          $ref: '#/components/responses/ErrorResponse'
        '404':
          $ref: '#/components/responses/ErrorResponse'
        '409':
          $ref: '#/components/responses/ErrorResponse'
        '500':
          $ref: '#/components/responses/ErrorResponse'
components:
  schemas:
    RefreshRequest:
      type: object
      required:
        - refresh_token
      properties:
        refresh_token:
          type: string
    AuthResponse:
      type: object
      required:
        - token_type
        - access_token
        - expires_in
        - refresh_token
        - refresh_expires_in
        - pop_key
      properties:
        access_token:
          type: string
        expires_in:
          type: integer
          format: int64
          minimum: 0
        pop_key:
          type: string
        refresh_expires_in:
          type: integer
          format: int64
          minimum: 0
        refresh_token:
          type: string
        token_type:
          type: string
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'

````

<!-- END SOURCE: api/auth/refresh-session.md -->

---

# Source: `api/exchange/build-register-trader-instructions.md`

**Original URL:** `https://docs.phoenix.trade/api/exchange/build-register-trader-instructions.md`

**Local path:** `api/exchange/build-register-trader-instructions.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Build register trader instructions

> Returns the register and delegated-onboarding instructions for the default trader account without a referral code. The caller chooses the transaction fee payer, builds the transaction locally, and signs it with the trader authority and fee payer.



## OpenAPI

````yaml /openapi/phoenix-public-api.json post /v1/exchange/build-register-ixs
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/exchange/build-register-ixs:
    post:
      tags:
        - Exchange
      summary: Build register trader instructions
      description: >-
        Returns the register and delegated-onboarding instructions for the
        default trader account without a referral code. The caller chooses the
        transaction fee payer, builds the transaction locally, and signs it with
        the trader authority and fee payer.
      operationId: post.v1.exchange.build_register_ixs
      requestBody:
        description: JSON request payload for `post.v1.exchange.build_register_ixs`.
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/BuildRegisterIxsRequest'
            example:
              traderAuthority: string
              txFeePayer: string
        required: true
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/BuildRegisterIxsResponse'
components:
  schemas:
    BuildRegisterIxsRequest:
      type: object
      description: Request payload for /exchange/build-register-ixs.
      required:
        - traderAuthority
        - txFeePayer
      properties:
        maxPositions:
          type:
            - integer
            - 'null'
          format: int32
          minimum: 0
        traderAuthority:
          type: string
        txFeePayer:
          type: string
    BuildRegisterIxsResponse:
      type: object
      description: Response payload for /exchange/build-register-ixs.
      required:
        - instructions
        - traderPda
        - traderOnboarder
        - txFeePayer
        - maxPositions
        - includeRegisterTrader
      properties:
        includeRegisterTrader:
          type: boolean
        instructions:
          type: array
          items:
            $ref: '#/components/schemas/ApiInstructionResponse'
        maxPositions:
          type: integer
          format: int32
          minimum: 0
        traderOnboarder:
          type: string
        traderPda:
          type: string
        txFeePayer:
          type: string
    ApiInstructionResponse:
      type: object
      description: |-
        API representation of a Solana instruction.

        Note: this is a pure DTO. When needed, clients/servers can enable the
        appropriate conversion logic in the consumer crate to avoid forcing a
        Solana dependency in `phoenix-api-types`.
      required:
        - data
        - keys
        - programId
      properties:
        data:
          type: array
          items:
            type: integer
            format: int32
            minimum: 0
        keys:
          type: array
          items:
            $ref: '#/components/schemas/ApiAccountMeta'
        programId:
          type: string
    ApiAccountMeta:
      type: object
      description: Account metadata returned from instruction-building endpoints.
      required:
        - pubkey
        - isSigner
        - isWritable
      properties:
        isSigner:
          type: boolean
        isWritable:
          type: boolean
        pubkey:
          type: string

````

<!-- END SOURCE: api/exchange/build-register-trader-instructions.md -->

---

# Source: `api/exchange/get-candles.md`

**Original URL:** `https://docs.phoenix.trade/api/exchange/get-candles.md`

**Local path:** `api/exchange/get-candles.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get candles

> Handles `GET /v1/candles/{symbol}` via `get.v1.candles.by_symbol`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/candles/{symbol}
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/candles/{symbol}:
    get:
      tags:
        - Exchange
      summary: Get candles
      description: Handles `GET /v1/candles/{symbol}` via `get.v1.candles.by_symbol`.
      operationId: get.v1.candles.by_symbol
      parameters:
        - name: symbol
          in: path
          description: Trading symbol
          required: true
          schema:
            type: string
        - name: timeframe
          in: query
          description: Timeframe
          required: true
          schema:
            type: string
        - name: startTime
          in: query
          description: Start time in milliseconds since Unix epoch
          required: false
          schema:
            type: integer
            format: int64
        - name: endTime
          in: query
          description: End time in milliseconds since Unix epoch
          required: false
          schema:
            type: integer
            format: int64
        - name: limit
          in: query
          description: 'Max number of candles (default: 2500)'
          required: false
          schema:
            type: integer
            format: int64
        - name: enableExternalSource
          in: query
          description: 'Opt-in external candles stored in the DB (default: false)'
          required: false
          schema:
            type: boolean
      responses:
        '200':
          description: Asset candles
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/ApiCandle'
        '400':
          $ref: '#/components/responses/ErrorResponse'
        '404':
          $ref: '#/components/responses/ErrorResponse'
        '500':
          $ref: '#/components/responses/ErrorResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    ApiCandle:
      type: object
      description: |-
        API Candle structure (Trading View Bar interface)
        Note: This is a simplified version. The full implementation with From
        conversions is in eternal-api/src/api/candles.rs to avoid heavy
        dependencies.
      required:
        - time
        - low
        - high
        - open
        - close
      properties:
        close:
          type: number
          format: double
        externalSource:
          type:
            - string
            - 'null'
          description: |-
            External candle source name (e.g., "binance", "coinbase").
            Omitted from JSON for exchange candles to preserve backward
            compatibility.
        high:
          type: number
          format: double
        low:
          type: number
          format: double
        markClose:
          type: number
          format: double
        markHigh:
          type: number
          format: double
        markLow:
          type: number
          format: double
        markOpen:
          type: number
          format: double
        open:
          type: number
          format: double
        time:
          type: integer
          format: int64
          description: Candle timestamp as a Unix timestamp in milliseconds (UTC).
        tradeCount:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        volume:
          type:
            - number
            - 'null'
          format: double
        volumeQuote:
          type:
            - number
            - 'null'
          format: double
          description: Quote currency volume (e.g. USDC) for the candle period.
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/exchange/get-candles.md -->

---

# Source: `api/exchange/get-commodity-market-calendar.md`

**Original URL:** `https://docs.phoenix.trade/api/exchange/get-commodity-market-calendar.md`

**Local path:** `api/exchange/get-commodity-market-calendar.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get commodity market calendar

> Handles `GET /v1/market/commodity-calendar` via `get.v1.market.commodity_calendar`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/market/commodity-calendar
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/market/commodity-calendar:
    get:
      tags:
        - Exchange
      summary: Get commodity market calendar
      description: >-
        Handles `GET /v1/market/commodity-calendar` via
        `get.v1.market.commodity_calendar`.
      operationId: get.v1.market.commodity_calendar
      responses:
        '200':
          description: Commodity market calendar
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/CommodityMarketCalendarResponse'
        '400':
          $ref: '#/components/responses/ErrorResponse'
        '404':
          $ref: '#/components/responses/ErrorResponse'
        '500':
          $ref: '#/components/responses/ErrorResponse'
        '503':
          $ref: '#/components/responses/ErrorResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    CommodityMarketCalendarResponse:
      type: object
      required:
        - market
        - loadedAt
        - rawJson
        - calendar
      properties:
        calendar:
          $ref: '#/components/schemas/CommodityMarketCalendarView'
        loadedAt:
          type: string
          format: date-time
        market:
          type: string
        rawJson:
          type: string
    CommodityMarketCalendarView:
      type: object
      required:
        - weeklySchedule
        - dateOverrides
      properties:
        dateOverrides:
          type: object
          additionalProperties:
            $ref: '#/components/schemas/CommodityMarketDaySchedule'
          propertyNames:
            type: string
        weeklySchedule:
          type: object
          additionalProperties:
            $ref: '#/components/schemas/CommodityMarketDaySchedule'
          propertyNames:
            type: string
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
    CommodityMarketDaySchedule:
      type: object
      required:
        - sessions
      properties:
        sessions:
          type: array
          items:
            $ref: '#/components/schemas/CommodityMarketSessionRange'
    CommodityMarketSessionRange:
      type: object
      description: A single named session within a [`CommodityMarketDaySchedule`].
      required:
        - start
        - end
        - mode
      properties:
        end:
          type: string
        mode:
          type: string
          description: Serialized calendar mode name, e.g. `"EXTERNAL"`, `"INTERNAL"`.
        start:
          type: string
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/exchange/get-commodity-market-calendar.md -->

---

# Source: `api/exchange/get-exchange-keys.md`

**Original URL:** `https://docs.phoenix.trade/api/exchange/get-exchange-keys.md`

**Local path:** `api/exchange/get-exchange-keys.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get exchange keys

> Handles `GET /v1/view/exchange/keys` via `get.v1.view.exchange.keys`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/view/exchange/keys
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/view/exchange/keys:
    get:
      tags:
        - Exchange
      summary: Get exchange keys
      description: Handles `GET /v1/view/exchange/keys` via `get.v1.view.exchange.keys`.
      operationId: get.v1.view.exchange.keys
      responses:
        '200':
          description: Exchange pubkeys
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ExchangeKeysView'
        '500':
          $ref: '#/components/responses/ErrorResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    ExchangeKeysView:
      type: object
      required:
        - globalConfig
        - currentAuthorities
        - pendingAuthorities
        - canonicalMint
        - globalVault
        - perpAssetMap
        - globalTraderIndex
        - activeTraderBuffer
        - withdrawQueue
      properties:
        activeTraderBuffer:
          type: array
          items:
            type: string
          description: Active trader buffer account public keys.
        canonicalMint:
          type: string
          description: Canonical quote mint public key.
        currentAuthorities:
          $ref: '#/components/schemas/AuthoritySetView'
          description: Currently active authority set.
        globalConfig:
          type: string
          description: Global configuration account public key.
        globalTraderIndex:
          type: array
          items:
            type: string
          description: Global trader index account public keys.
        globalVault:
          type: string
          description: Global vault public key.
        pendingAuthorities:
          $ref: '#/components/schemas/AuthoritySetView'
          description: Pending authority set awaiting activation.
        perpAssetMap:
          type: string
          description: Perp asset map public key.
        withdrawQueue:
          type: string
          description: Withdraw queue account public key.
    AuthoritySetView:
      type: object
      description: View for authority set containing all authority pubkeys
      required:
        - rootAuthority
        - riskAuthority
        - marketAuthority
        - oracleAuthority
        - adlAuthority
        - cancelAuthority
        - backstopAuthority
      properties:
        adlAuthority:
          type: string
          description: ADL authority public key.
        backstopAuthority:
          type: string
          description: Backstop authority public key.
        cancelAuthority:
          type: string
          description: Cancel authority public key.
        marketAuthority:
          type: string
          description: Market authority public key.
        oracleAuthority:
          type: string
          description: Oracle authority public key.
        riskAuthority:
          type: string
          description: Risk authority public key.
        rootAuthority:
          type: string
          description: Root authority public key.
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/exchange/get-exchange-keys.md -->

---

# Source: `api/exchange/get-exchange-snapshot.md`

**Original URL:** `https://docs.phoenix.trade/api/exchange/get-exchange-snapshot.md`

**Local path:** `api/exchange/get-exchange-snapshot.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get exchange snapshot

> Handles `GET /v1/exchange/snapshot` via `get.v1.exchange.snapshot`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/exchange/snapshot
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/exchange/snapshot:
    get:
      tags:
        - Exchange
      summary: Get exchange snapshot
      description: Handles `GET /v1/exchange/snapshot` via `get.v1.exchange.snapshot`.
      operationId: get.v1.exchange.snapshot
      responses:
        '200':
          description: Exchange snapshot
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ExchangeSnapshotView'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    ExchangeSnapshotView:
      type: object
      required:
        - version
        - slot
        - slotIndex
        - exchange
        - markets
      properties:
        exchange:
          $ref: '#/components/schemas/ExchangeStateSnapshot'
        markets:
          type: array
          items:
            $ref: '#/components/schemas/ExchangeMarketSnapshot'
        sequenceNumber:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/JsSafeU64'
        slot:
          type: integer
          format: int64
          minimum: 0
        slotIndex:
          type: integer
          format: int32
          minimum: 0
        version:
          type: integer
          format: int32
          minimum: 0
    ExchangeStateSnapshot:
      type: object
      required:
        - programId
        - globalConfig
        - currentAuthorities
        - canonicalMint
        - usdcMint
        - globalVault
        - perpAssetMap
        - globalTraderIndex
        - activeTraderBuffer
        - withdrawQueue
        - exchangeStatusBits
        - exchangeStatusFeatures
        - active
        - gated
      properties:
        active:
          type: boolean
        activeTraderBuffer:
          type: array
          items:
            type: string
        canonicalMint:
          type: string
        currentAuthorities:
          $ref: '#/components/schemas/AuthoritySetView'
        exchangeStatusBits:
          type: integer
          format: int32
          minimum: 0
        exchangeStatusFeatures:
          type: array
          items:
            type: string
        gated:
          type: boolean
        globalConfig:
          type: string
        globalTraderIndex:
          type: array
          items:
            type: string
        globalVault:
          type: string
        perpAssetMap:
          type: string
        programId:
          type: string
        usdcMint:
          type: string
        withdrawQueue:
          type: string
        withdrawalsAvailable:
          type: boolean
    ExchangeMarketSnapshot:
      type: object
      required:
        - symbol
        - assetId
        - marketStatus
        - marketPubkey
        - splinePubkey
        - tickSize
        - baseLotsDecimals
        - takerFee
        - makerFee
        - leverageTiers
        - riskFactors
        - fundingConfig
        - openInterestCapBaseLots
        - maxLiquidationSizeBaseLots
        - isolatedOnly
        - markPriceParameters
      properties:
        assetId:
          type: integer
          format: int32
          minimum: 0
        baseLotsDecimals:
          type: integer
          format: int32
        commodityMetadata:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/ExchangeWsCommodityMetadata'
        fundingConfig:
          $ref: '#/components/schemas/ExchangeWsFundingConfig'
        isolatedOnly:
          type: boolean
        leverageTiers:
          type: array
          items:
            $ref: '#/components/schemas/ExchangeWsLeverageTier'
        makerFee:
          type: number
          format: double
        markPriceParameters:
          $ref: '#/components/schemas/ExchangeWsMarkPriceParameters'
        marketPubkey:
          type: string
        marketStatus:
          $ref: '#/components/schemas/MarketStatus'
        maxLiquidationSizeBaseLots:
          $ref: '#/components/schemas/JsSafeU64'
        metadata:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/MarketPublicMetadata'
        openInterestCapBaseLots:
          $ref: '#/components/schemas/JsSafeU64'
        riskFactors:
          $ref: '#/components/schemas/ExchangeWsRiskFactors'
        splinePubkey:
          type: string
        symbol:
          type: string
        takerFee:
          type: number
          format: double
        tickSize:
          type: integer
          format: int64
          minimum: 0
    JsSafeU64:
      type: integer
      format: int64
      description: |-
        Wrapper for unsigned 64-bit values that must be JSON-safe for consumers
        written in JavaScript/TypeScript. Mirrors [`JsSafeI64`] but for unsigned
        Phoenix quantities such as base lots, quote lots, and slots.
      minimum: 0
    AuthoritySetView:
      type: object
      description: View for authority set containing all authority pubkeys
      required:
        - rootAuthority
        - riskAuthority
        - marketAuthority
        - oracleAuthority
        - adlAuthority
        - cancelAuthority
        - backstopAuthority
      properties:
        adlAuthority:
          type: string
          description: ADL authority public key.
        backstopAuthority:
          type: string
          description: Backstop authority public key.
        cancelAuthority:
          type: string
          description: Cancel authority public key.
        marketAuthority:
          type: string
          description: Market authority public key.
        oracleAuthority:
          type: string
          description: Oracle authority public key.
        riskAuthority:
          type: string
          description: Risk authority public key.
        rootAuthority:
          type: string
          description: Root authority public key.
    ExchangeWsCommodityMetadata:
      type: object
      required:
        - isCommodity
        - isReopen
        - isAfterHours
        - status
        - afterHoursRadius
      properties:
        afterHoursRadius:
          type: string
        executionPriceBand:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/ExchangeWsMarketPriceBand'
        isAfterHours:
          type: boolean
        isCommodity:
          type: boolean
        isReopen:
          type: boolean
        lastIndexExpiryTimestamp:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        lastKnownIndexPrice:
          type:
            - string
            - 'null'
        markPriceBand:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/ExchangeWsMarketPriceBand'
        status:
          $ref: '#/components/schemas/CommodityMarketState'
    ExchangeWsFundingConfig:
      type: object
      required:
        - fundingIntervalSeconds
        - fundingPeriodSeconds
        - maxFundingRatePerInterval
      properties:
        fundingIntervalSeconds:
          type: integer
          format: int64
          minimum: 0
        fundingPeriodSeconds:
          type: integer
          format: int64
          minimum: 0
        maxFundingRatePerInterval:
          type: integer
          format: int64
    ExchangeWsLeverageTier:
      type: object
      required:
        - maxLeverage
        - maxSizeBaseLots
        - limitOrderRiskFactor
      properties:
        limitOrderRiskFactor:
          type: integer
          format: int32
          minimum: 0
        maxLeverage:
          type: integer
          format: int64
          minimum: 0
        maxSizeBaseLots:
          $ref: '#/components/schemas/JsSafeU64'
    ExchangeWsMarkPriceParameters:
      type: object
      required:
        - emaPeriodSlots
        - emaDiffRadius
        - bookPriceRadius
        - commoditiesAfterHoursRadius
        - commoditiesAfterHoursRadiusBps
        - adjustedExchangeSpotPriceWeight
        - bookPriceWeight
        - exchangePerpPriceWeight
        - spotPriceStaleThreshold
        - bookPriceStaleThreshold
        - perpPriceStaleThreshold
        - riskActionPriceValidityRules
        - oracleDivergenceRadius
        - minOracleResponses
      properties:
        adjustedExchangeSpotPriceWeight:
          $ref: '#/components/schemas/JsSafeU64'
        bookHardStaleMultiplier:
          type: integer
          format: int32
          minimum: 0
        bookPriceRadius:
          $ref: '#/components/schemas/JsSafeU64'
        bookPriceStaleThreshold:
          $ref: '#/components/schemas/JsSafeU64'
        bookPriceWeight:
          $ref: '#/components/schemas/JsSafeU64'
        commoditiesAfterHoursRadius:
          $ref: '#/components/schemas/JsSafeU64'
        commoditiesAfterHoursRadiusBps:
          $ref: '#/components/schemas/JsSafeU64'
        emaDiffRadius:
          $ref: '#/components/schemas/JsSafeU64'
        emaPeriodSlots:
          $ref: '#/components/schemas/JsSafeU64'
        exchangePerpPriceWeight:
          $ref: '#/components/schemas/JsSafeU64'
        minOracleResponses:
          type: integer
          format: int32
          minimum: 0
        oracleDivergenceRadius:
          type: integer
          format: int32
          minimum: 0
        oracleHardStaleMultiplier:
          type: integer
          format: int32
          minimum: 0
        perpPriceStaleThreshold:
          $ref: '#/components/schemas/JsSafeU64'
        riskActionPriceValidityRules:
          type: array
          items:
            type: array
            items:
              type: array
              items:
                $ref: '#/components/schemas/ExchangeWsValidationRule'
        spotPriceStaleThreshold:
          $ref: '#/components/schemas/JsSafeU64'
    MarketStatus:
      type: string
      enum:
        - uninitialized
        - active
        - postOnly
        - paused
        - closed
        - tombstoned
    MarketPublicMetadata:
      type: object
      description: Public off-chain metadata for a market.
      properties:
        calendar:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/MarketCalendar'
              description: >-
                Market calendar metadata derived from the configured calendar
                id.
        coinGeckoId:
          type:
            - string
            - 'null'
          description: CoinGecko asset identifier.
        coinMarketCapId:
          type:
            - integer
            - 'null'
          format: int64
          description: CoinMarketCap numeric asset identifier.
        description:
          type:
            - string
            - 'null'
          description: Human-readable market description.
        displayColor:
          type:
            - string
            - 'null'
          description: Preferred display color for this market.
        logoUri:
          type:
            - string
            - 'null'
          description: Logo URI for this market.
        name:
          type:
            - string
            - 'null'
          description: Human-readable market name.
        tokensXyzAssetId:
          type:
            - string
            - 'null'
          description: tokens.xyz asset identifier.
    ExchangeWsRiskFactors:
      type: object
      required:
        - maintenance
        - backstop
        - highRisk
        - upnl
        - upnlForWithdrawals
        - cancelOrder
      properties:
        backstop:
          type: number
          format: double
          description: Backstop liquidation risk factor as a percentage.
        backstopBps:
          type:
            - integer
            - 'null'
          format: int32
          description: Backstop liquidation risk factor in basis points.
          minimum: 0
        cancelOrder:
          type: number
          format: double
          description: Cancel order risk factor as a percentage.
        cancelOrderBps:
          type:
            - integer
            - 'null'
          format: int32
          description: Cancel order risk factor in basis points.
          minimum: 0
        highRisk:
          type: number
          format: double
          description: High-risk threshold as a percentage.
        highRiskBps:
          type:
            - integer
            - 'null'
          format: int32
          description: High-risk threshold in basis points.
          minimum: 0
        maintenance:
          type: number
          format: double
          description: Maintenance margin risk factor as a percentage (e.g., 50.0 = 50%).
        maintenanceBps:
          type:
            - integer
            - 'null'
          format: int32
          description: Maintenance margin risk factor in basis points (e.g., 5000 = 50%).
          minimum: 0
        upnl:
          type: number
          format: double
          description: Risk factor for positive unrealized PnL penalty as a percentage.
        upnlBps:
          type:
            - integer
            - 'null'
          format: int32
          description: Risk factor for positive unrealized PnL penalty in basis points.
          minimum: 0
        upnlForWithdrawals:
          type: number
          format: double
          description: >-
            Risk factor for positive unrealized PnL penalty during withdrawals
            in

            percentage terms.
        upnlForWithdrawalsBps:
          type:
            - integer
            - 'null'
          format: int32
          description: >-
            Risk factor for positive unrealized PnL penalty during withdrawals
            in

            basis points.
          minimum: 0
    ExchangeWsMarketPriceBand:
      type: object
      required:
        - lower
        - upper
      properties:
        lower:
          type: string
        upper:
          type: string
    CommodityMarketState:
      type: string
      description: >-
        Represents the state of a RWA market. This type is shared between
        on-chain

        and off-chain components
      enum:
        - active
        - afterHours
        - reopen
    ExchangeWsValidationRule:
      type: string
      enum:
        - ignore
        - require
        - forbid
    MarketCalendar:
      type: object
      description: Public metadata for a market calendar associated with a market.
      required:
        - id
        - description
        - calendarUri
        - contentSha256
      properties:
        calendarUri:
          type: string
          description: URI where the full market calendar can be fetched.
        contentSha256:
          type: string
          description: SHA-256 hash of the calendar content.
        description:
          type: string
          description: Human-readable calendar description.
        id:
          type: string
          description: Market calendar identifier configured off-chain.
        nextMarketTransitionUtc:
          type:
            - string
            - 'null'
          format: date-time
          description: Next UTC timestamp at which this calendar changes market state.
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/exchange/get-exchange-snapshot.md -->

---

# Source: `api/exchange/get-exchange-status.md`

**Original URL:** `https://docs.phoenix.trade/api/exchange/get-exchange-status.md`

**Local path:** `api/exchange/get-exchange-status.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get exchange status

> Handles `GET /v1/view/exchange/status` via `get.v1.view.exchange.status`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/view/exchange/status
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/view/exchange/status:
    get:
      tags:
        - Exchange
      summary: Get exchange status
      description: >-
        Handles `GET /v1/view/exchange/status` via
        `get.v1.view.exchange.status`.
      operationId: get.v1.view.exchange.status
      responses:
        '200':
          description: Exchange status
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ExchangeStatusView'
        '500':
          $ref: '#/components/responses/ErrorResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    ExchangeStatusView:
      type: object
      description: |-
        Exchange status view containing the current operational status of the
        exchange.
      required:
        - active
        - gated
      properties:
        active:
          type: boolean
          description: Whether the exchange is active (accepting orders)
        gated:
          type: boolean
          description: Whether the exchange is in gated mode (restricted access)
        withdrawalsAvailable:
          type: boolean
          description: Whether withdrawals are currently available.
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/exchange/get-exchange-status.md -->

---

# Source: `api/exchange/get-exchange.md`

**Original URL:** `https://docs.phoenix.trade/api/exchange/get-exchange.md`

**Local path:** `api/exchange/get-exchange.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get exchange

> Handles `GET /v1/view/exchange` via `get.v1.view.exchange`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/view/exchange
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/view/exchange:
    get:
      tags:
        - Exchange
      summary: Get exchange
      description: Handles `GET /v1/view/exchange` via `get.v1.view.exchange`.
      operationId: get.v1.view.exchange
      responses:
        '200':
          description: Phoenix Eternal exchange configuration
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ExchangeView'
        '500':
          $ref: '#/components/responses/ErrorResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    ExchangeView:
      type: object
      description: |-
        Full exchange configuration response containing keys and market configs.
        Does NOT include live data like mark prices or current open interest.
      required:
        - keys
        - markets
      properties:
        keys:
          $ref: '#/components/schemas/ExchangeKeysView'
          description: Exchange-level account keys and authorities.
        markets:
          type: array
          items:
            $ref: '#/components/schemas/ExchangeMarketConfig'
          description: Static per-market configuration entries.
    ExchangeKeysView:
      type: object
      required:
        - globalConfig
        - currentAuthorities
        - pendingAuthorities
        - canonicalMint
        - globalVault
        - perpAssetMap
        - globalTraderIndex
        - activeTraderBuffer
        - withdrawQueue
      properties:
        activeTraderBuffer:
          type: array
          items:
            type: string
          description: Active trader buffer account public keys.
        canonicalMint:
          type: string
          description: Canonical quote mint public key.
        currentAuthorities:
          $ref: '#/components/schemas/AuthoritySetView'
          description: Currently active authority set.
        globalConfig:
          type: string
          description: Global configuration account public key.
        globalTraderIndex:
          type: array
          items:
            type: string
          description: Global trader index account public keys.
        globalVault:
          type: string
          description: Global vault public key.
        pendingAuthorities:
          $ref: '#/components/schemas/AuthoritySetView'
          description: Pending authority set awaiting activation.
        perpAssetMap:
          type: string
          description: Perp asset map public key.
        withdrawQueue:
          type: string
          description: Withdraw queue account public key.
    ExchangeMarketConfig:
      type: object
      description: >-
        Static market configuration without live data like prices or open
        interest.

        Used by the `/exchange` endpoint to return market parameters.
      required:
        - symbol
        - assetId
        - marketStatus
        - marketPubkey
        - splinePubkey
        - tickSize
        - baseLotsDecimals
        - takerFee
        - makerFee
        - leverageTiers
        - riskFactors
        - fundingIntervalSeconds
        - fundingPeriodSeconds
        - maxFundingRatePerInterval
        - openInterestCapBaseLots
        - maxLiquidationSizeBaseLots
        - isolatedOnly
      properties:
        assetId:
          type: integer
          format: int32
          description: Numeric asset identifier.
          minimum: 0
        baseLotsDecimals:
          type: integer
          format: int32
          description: Base-lot decimal exponent.
        commodityMetadata:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/CommodityMetadata'
              description: Commodity-specific metadata, present only for commodity markets.
        fundingIntervalSeconds:
          type: integer
          format: int32
          description: Funding interval length in seconds.
          minimum: 0
        fundingPeriodSeconds:
          type: integer
          format: int32
          description: Funding period length in seconds.
          minimum: 0
        isolatedOnly:
          type: boolean
          description: Whether this market only supports isolated margin positions
        leverageTiers:
          type: array
          items:
            $ref: '#/components/schemas/ExchangeLeverageTier'
          description: Configured leverage tiers.
        makerFee:
          type: number
          format: double
          description: Maker fee (percent).
        marketPubkey:
          type: string
          description: The orderbook account pubkey (base58 encoded)
        marketStatus:
          $ref: '#/components/schemas/MarketStatus'
          description: Current market status.
        maxFundingRatePerInterval:
          type: integer
          format: int64
          description: Maximum absolute funding rate per interval.
        maxFundingRatePerIntervalPercentage:
          type: number
          format: double
          description: >-
            Maximum absolute funding rate per interval as a percentage of
            notional

            at the current mark price.
        maxLiquidationSizeBaseLots:
          $ref: '#/components/schemas/JsSafeU64'
          description: Maximum size for a single liquidation (in base lots)
        metadata:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/MarketPublicMetadata'
              description: Public market metadata configured off-chain.
        openInterestCapBaseLots:
          $ref: '#/components/schemas/JsSafeU64'
          description: Maximum open interest allowed for this market (in base lots)
        riskFactors:
          $ref: '#/components/schemas/ExchangeRiskFactors'
          description: Risk factors as percentages.
        splinePubkey:
          type: string
          description: The spline collection PDA (derived from market_pubkey)
        statsSnapshot:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/MarketStatsSnapshot'
              description: >-
                Optional live stats snapshot for seeding clients before
                websocket

                updates.
        symbol:
          type: string
          description: Market symbol (for example, "SOL-PERP").
        takerFee:
          type: number
          format: double
          description: Taker fee (percent).
        tickSize:
          type: integer
          format: int64
          description: Tick size in quote lots per base lot per tick.
          minimum: 0
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
    AuthoritySetView:
      type: object
      description: View for authority set containing all authority pubkeys
      required:
        - rootAuthority
        - riskAuthority
        - marketAuthority
        - oracleAuthority
        - adlAuthority
        - cancelAuthority
        - backstopAuthority
      properties:
        adlAuthority:
          type: string
          description: ADL authority public key.
        backstopAuthority:
          type: string
          description: Backstop authority public key.
        cancelAuthority:
          type: string
          description: Cancel authority public key.
        marketAuthority:
          type: string
          description: Market authority public key.
        oracleAuthority:
          type: string
          description: Oracle authority public key.
        riskAuthority:
          type: string
          description: Risk authority public key.
        rootAuthority:
          type: string
          description: Root authority public key.
    CommodityMetadata:
      type: object
      description: Commodity-specific market metadata.
      required:
        - isCommodity
        - isReopen
        - isAfterHours
        - status
        - afterHoursRadius
      properties:
        afterHoursRadius:
          type: string
          description: Commodity after-hours radius in price units.
        executionPriceBand:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/MarketPriceBand'
              description: >-
                Price band currently applied to execution-visible orderbook
                levels.
        isAfterHours:
          type: boolean
          description: Parsed after-hours flag.
        isCommodity:
          type: boolean
          description: Parsed commodity flag. Always true when this object is present.
        isReopen:
          type: boolean
          description: Parsed reopen flag.
        lastIndexExpiryTimestamp:
          type:
            - integer
            - 'null'
          format: int64
          description: Unix timestamp when the last known index price expires.
          minimum: 0
        lastKnownIndexPrice:
          type:
            - string
            - 'null'
          description: Last index price used for commodity after-hours/reopen behavior.
        markPriceBand:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/MarketPriceBand'
              description: Mark-price clamp band around the last known index price.
        status:
          $ref: '#/components/schemas/CommodityMarketState'
          description: Derived commodity state.
    ExchangeLeverageTier:
      type: object
      description: |-
        Leverage tier with risk factors as f64 percentages.
        Used by the `/exchange` endpoint.
      required:
        - maxLeverage
        - maxSizeBaseLots
        - limitOrderRiskFactor
      properties:
        limitOrderRiskFactor:
          type: number
          format: double
          description: The limit order risk factor as a percentage (e.g., 60.0 = 60%).
        limitOrderRiskFactorBps:
          type:
            - integer
            - 'null'
          format: int32
          description: The limit order risk factor in basis points (e.g., 6000 = 60%).
          minimum: 0
        maxLeverage:
          type: number
          format: double
          description: Maximum leverage for this tier.
        maxSizeBaseLots:
          type: integer
          format: int64
          description: Maximum size in base lots for this tier.
          minimum: 0
    MarketStatus:
      type: string
      enum:
        - uninitialized
        - active
        - postOnly
        - paused
        - closed
        - tombstoned
    JsSafeU64:
      type: integer
      format: int64
      description: |-
        Wrapper for unsigned 64-bit values that must be JSON-safe for consumers
        written in JavaScript/TypeScript. Mirrors [`JsSafeI64`] but for unsigned
        Phoenix quantities such as base lots, quote lots, and slots.
      minimum: 0
    MarketPublicMetadata:
      type: object
      description: Public off-chain metadata for a market.
      properties:
        calendar:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/MarketCalendar'
              description: >-
                Market calendar metadata derived from the configured calendar
                id.
        coinGeckoId:
          type:
            - string
            - 'null'
          description: CoinGecko asset identifier.
        coinMarketCapId:
          type:
            - integer
            - 'null'
          format: int64
          description: CoinMarketCap numeric asset identifier.
        description:
          type:
            - string
            - 'null'
          description: Human-readable market description.
        displayColor:
          type:
            - string
            - 'null'
          description: Preferred display color for this market.
        logoUri:
          type:
            - string
            - 'null'
          description: Logo URI for this market.
        name:
          type:
            - string
            - 'null'
          description: Human-readable market name.
        tokensXyzAssetId:
          type:
            - string
            - 'null'
          description: tokens.xyz asset identifier.
    ExchangeRiskFactors:
      type: object
      description: |-
        Risk factors as percentages.
        Used by the `/exchange` endpoint.
      required:
        - maintenance
        - backstop
        - highRisk
        - upnl
        - upnlForWithdrawals
        - cancelOrder
      properties:
        backstop:
          type: number
          format: double
          description: Backstop liquidation risk factor as a percentage.
        backstopBps:
          type:
            - integer
            - 'null'
          format: int32
          description: Backstop liquidation risk factor in basis points.
          minimum: 0
        cancelOrder:
          type: number
          format: double
          description: Cancel order risk factor as a percentage.
        cancelOrderBps:
          type:
            - integer
            - 'null'
          format: int32
          description: Cancel order risk factor in basis points.
          minimum: 0
        highRisk:
          type: number
          format: double
          description: High risk threshold as a percentage.
        highRiskBps:
          type:
            - integer
            - 'null'
          format: int32
          description: High risk threshold in basis points.
          minimum: 0
        maintenance:
          type: number
          format: double
          description: Maintenance margin risk factor as a percentage (e.g., 50.0 = 50%).
        maintenanceBps:
          type:
            - integer
            - 'null'
          format: int32
          description: Maintenance margin risk factor in basis points (e.g., 5000 = 50%).
          minimum: 0
        upnl:
          type: number
          format: double
          description: Risk factor for positive unrealized PnL penalty as a percentage.
        upnlBps:
          type:
            - integer
            - 'null'
          format: int32
          description: Risk factor for positive unrealized PnL penalty in basis points.
          minimum: 0
        upnlForWithdrawals:
          type: number
          format: double
          description: >-
            Risk factor for positive unrealized PnL penalty during withdrawals
            as a

            percentage.
        upnlForWithdrawalsBps:
          type:
            - integer
            - 'null'
          format: int32
          description: >-
            Risk factor for positive unrealized PnL penalty during withdrawals
            in

            basis points.
          minimum: 0
    MarketStatsSnapshot:
      type: object
      description: Live market stats captured alongside an exchange market config response.
      required:
        - slot
        - slotIndex
        - openInterestBaseLots
        - fundingStartIntervalTimestamp
        - cumulativeFundingRate
      properties:
        cumulativeFundingRate:
          $ref: '#/components/schemas/JsSafeI64'
          description: Current cumulative funding rate.
        fundingStartIntervalTimestamp:
          $ref: '#/components/schemas/JsSafeU64'
          description: Unix timestamp in seconds for the current funding interval start.
        openInterestBaseLots:
          $ref: '#/components/schemas/JsSafeU64'
          description: Current open interest in base lots.
        slot:
          type: integer
          format: int64
          description: Solana slot of the state snapshot used to build these stats.
          minimum: 0
        slotIndex:
          type: integer
          format: int32
          description: |-
            Intra-slot sequence index of the state snapshot used to build these
            stats.
          minimum: 0
    MarketPriceBand:
      type: object
      description: Inclusive lower/upper price bounds for market-specific execution rules.
      required:
        - min
        - max
      properties:
        max:
          type: string
        min:
          type: string
    CommodityMarketState:
      type: string
      description: >-
        Represents the state of a RWA market. This type is shared between
        on-chain

        and off-chain components
      enum:
        - active
        - afterHours
        - reopen
    MarketCalendar:
      type: object
      description: Public metadata for a market calendar associated with a market.
      required:
        - id
        - description
        - calendarUri
        - contentSha256
      properties:
        calendarUri:
          type: string
          description: URI where the full market calendar can be fetched.
        contentSha256:
          type: string
          description: SHA-256 hash of the calendar content.
        description:
          type: string
          description: Human-readable calendar description.
        id:
          type: string
          description: Market calendar identifier configured off-chain.
        nextMarketTransitionUtc:
          type:
            - string
            - 'null'
          format: date-time
          description: Next UTC timestamp at which this calendar changes market state.
    JsSafeI64:
      type: integer
      format: int64
      description: >-
        Wrapper for signed 64-bit values that need to survive JSON transport
        without

        tripping JavaScript's safe-integer limits. When `serde` is enabled, this
        can

        deserialize from a string or number to accommodate clients that
        stringify

        large values.
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/exchange/get-exchange.md -->

---

# Source: `api/exchange/get-funding-overview.md`

**Original URL:** `https://docs.phoenix.trade/api/exchange/get-funding-overview.md`

**Local path:** `api/exchange/get-funding-overview.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get funding overview

> Handles `GET /v1/funding/overview` via `get.v1.funding.overview`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/funding/overview
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/funding/overview:
    get:
      tags:
        - Exchange
      summary: Get funding overview
      description: Handles `GET /v1/funding/overview` via `get.v1.funding.overview`.
      operationId: get.v1.funding.overview
      parameters:
        - name: startTime
          in: query
          description: >-
            Start time in milliseconds since Unix epoch (default: now - 7 days,
            max range: 1 year)
          required: false
          schema:
            type: integer
            format: int64
        - name: endTime
          in: query
          description: 'End time in milliseconds since Unix epoch (default: now)'
          required: false
          schema:
            type: integer
            format: int64
        - name: perMarketLimit
          in: query
          description: 'Max number of points to return per market (default: 500, max: 5000)'
          required: false
          schema:
            type: integer
            format: int64
      responses:
        '200':
          description: Funding overview history grouped by market
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/FundingOverviewResponse'
        '400':
          $ref: '#/components/responses/ErrorResponse'
        '500':
          $ref: '#/components/responses/ErrorResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    FundingOverviewResponse:
      type: object
      description: Response containing funding overview series for multiple markets
      required:
        - series
      properties:
        series:
          type: array
          items:
            $ref: '#/components/schemas/FundingOverviewSeries'
    FundingOverviewSeries:
      type: object
      description: Funding overview series for a single market
      required:
        - marketId
        - symbol
        - points
      properties:
        marketId:
          type: integer
          format: int64
        points:
          type: array
          items:
            $ref: '#/components/schemas/FundingOverviewPoint'
        symbol:
          type: string
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
    FundingOverviewPoint:
      type: object
      description: Individual funding overview data point for a market
      required:
        - timestamp
        - fundingAmountPerUnit
        - markPrice
        - fundingRate
      properties:
        fundingAmountPerUnit:
          type: string
          description: Funding amount per base unit in quote units.
        fundingRate:
          type: string
          description: Funding rate for the interval as a decimal.
        markPrice:
          type: string
          description: Mark price in quote units per base unit.
        timestamp:
          type: string
          format: date-time
          description: Formatted datetime string (ISO 8601).
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/exchange/get-funding-overview.md -->

---

# Source: `api/exchange/get-funding-rate-history.md`

**Original URL:** `https://docs.phoenix.trade/api/exchange/get-funding-rate-history.md`

**Local path:** `api/exchange/get-funding-rate-history.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get funding rate history

> Handles `GET /v1/funding/{symbol}/rates` via `get.v1.funding.by_symbol.rates`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/funding/{symbol}/rates
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/funding/{symbol}/rates:
    get:
      tags:
        - Exchange
      summary: Get funding rate history
      description: >-
        Handles `GET /v1/funding/{symbol}/rates` via
        `get.v1.funding.by_symbol.rates`.
      operationId: get.v1.funding.by_symbol.rates
      parameters:
        - name: symbol
          in: path
          description: Market symbol (e.g., 'SOL-PERP')
          required: true
          schema:
            type: string
        - name: startTime
          in: query
          description: >-
            Start time in milliseconds since Unix epoch (default: now - 7 days,
            max range: 1 year)
          required: false
          schema:
            type: integer
            format: int64
        - name: endTime
          in: query
          description: 'End time in milliseconds since Unix epoch (default: now)'
          required: false
          schema:
            type: integer
            format: int64
        - name: limit
          in: query
          description: 'Max number of funding rate points (default: 1000, max: 10000)'
          required: false
          schema:
            type: integer
            format: int64
      responses:
        '200':
          description: Funding rate history
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/FundingRateHistoryResponse'
        '400':
          $ref: '#/components/responses/ErrorResponse'
        '404':
          $ref: '#/components/responses/ErrorResponse'
        '500':
          $ref: '#/components/responses/ErrorResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    FundingRateHistoryResponse:
      type: object
      description: Response containing historical funding rates
      required:
        - marketId
        - symbol
        - rates
      properties:
        marketId:
          type: integer
          format: int64
        rates:
          type: array
          items:
            $ref: '#/components/schemas/FundingRatePoint'
        symbol:
          type: string
    FundingRatePoint:
      type: object
      description: Individual funding rate data point
      required:
        - timestamp
        - fundingRatePercentage
      properties:
        fundingRatePercentage:
          type: string
          description: >-
            Funding rate applied for the interval (percentage, USD notional
            terms)
        timestamp:
          type: string
          format: date-time
          description: Formatted datetime string (ISO 8601).
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/exchange/get-funding-rate-history.md -->

---

# Source: `api/exchange/get-mark-price.md`

**Original URL:** `https://docs.phoenix.trade/api/exchange/get-mark-price.md`

**Local path:** `api/exchange/get-mark-price.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get mark price

> Handles `GET /v1/market/{symbol}/mark-price` via `get.v1.market.by_symbol.mark_price`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/market/{symbol}/mark-price
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/market/{symbol}/mark-price:
    get:
      tags:
        - Exchange
      summary: Get mark price
      description: >-
        Handles `GET /v1/market/{symbol}/mark-price` via
        `get.v1.market.by_symbol.mark_price`.
      operationId: get.v1.market.by_symbol.mark_price
      parameters:
        - name: symbol
          in: path
          description: Market symbol
          required: true
          schema:
            type: string
      responses:
        '200':
          description: Mark price
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/MarkPriceResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    MarkPriceResponse:
      type: object
      required:
        - slot
        - slotIndex
        - symbol
      properties:
        markPrice:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/Price'
        slot:
          type: integer
          format: int64
          minimum: 0
        slotIndex:
          type: integer
          format: int32
          minimum: 0
        symbol:
          type: string
    Price:
      type: object
      description: A price for an asset.
      required:
        - price
        - slot
      properties:
        price:
          type: number
          format: double
        slot:
          type: integer
          format: int64
          minimum: 0
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/exchange/get-mark-price.md -->

---

# Source: `api/exchange/get-market-calendar-by-id.md`

**Original URL:** `https://docs.phoenix.trade/api/exchange/get-market-calendar-by-id.md`

**Local path:** `api/exchange/get-market-calendar-by-id.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get market calendar by ID

> Handles `GET /v1/market-calendar/{market_calendar_id}` via `get.v1.market_calendar.by_market_calendar_id`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/market-calendar/{market_calendar_id}
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/market-calendar/{market_calendar_id}:
    get:
      tags:
        - Exchange
      summary: Get market calendar by ID
      description: >-
        Handles `GET /v1/market-calendar/{market_calendar_id}` via
        `get.v1.market_calendar.by_market_calendar_id`.
      operationId: get.v1.market_calendar.by_market_calendar_id
      parameters:
        - name: market_calendar_id
          in: path
          description: Market calendar id
          required: true
          schema:
            type: string
      responses:
        '200':
          description: Market calendar
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/MarketCalendarRecord'
        '404':
          $ref: '#/components/responses/ErrorResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    MarketCalendarRecord:
      type: object
      required:
        - marketCalendarId
        - kind
        - description
        - s3Path
        - calendarUri
        - contentSha256
        - loadedAt
        - updatedAt
        - rawJson
      properties:
        calendar:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/CommodityMarketCalendarView'
        calendarUri:
          type: string
        contentSha256:
          type: string
        description:
          type: string
        kind:
          type: string
        loadedAt:
          type: string
          format: date-time
        marketCalendarId:
          type: string
        rawJson:
          type: string
        s3Path:
          type: string
        updatedAt:
          type: string
          format: date-time
    CommodityMarketCalendarView:
      type: object
      required:
        - weeklySchedule
        - dateOverrides
      properties:
        dateOverrides:
          type: object
          additionalProperties:
            $ref: '#/components/schemas/CommodityMarketDaySchedule'
          propertyNames:
            type: string
        weeklySchedule:
          type: object
          additionalProperties:
            $ref: '#/components/schemas/CommodityMarketDaySchedule'
          propertyNames:
            type: string
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
    CommodityMarketDaySchedule:
      type: object
      required:
        - sessions
      properties:
        sessions:
          type: array
          items:
            $ref: '#/components/schemas/CommodityMarketSessionRange'
    CommodityMarketSessionRange:
      type: object
      description: A single named session within a [`CommodityMarketDaySchedule`].
      required:
        - start
        - end
        - mode
      properties:
        end:
          type: string
        mode:
          type: string
          description: Serialized calendar mode name, e.g. `"EXTERNAL"`, `"INTERNAL"`.
        start:
          type: string
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/exchange/get-market-calendar-by-id.md -->

---

# Source: `api/exchange/get-market-calendar.md`

**Original URL:** `https://docs.phoenix.trade/api/exchange/get-market-calendar.md`

**Local path:** `api/exchange/get-market-calendar.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get market calendar

> Handles `GET /v1/market/{symbol}/market-calendar` via `get.v1.market.by_symbol.market_calendar`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/market/{symbol}/market-calendar
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/market/{symbol}/market-calendar:
    get:
      tags:
        - Exchange
      summary: Get market calendar
      description: >-
        Handles `GET /v1/market/{symbol}/market-calendar` via
        `get.v1.market.by_symbol.market_calendar`.
      operationId: get.v1.market.by_symbol.market_calendar
      parameters:
        - name: symbol
          in: path
          description: Market symbol
          required: true
          schema:
            type: string
      responses:
        '200':
          description: Market calendar
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/MarketCalendarResponse'
        '400':
          $ref: '#/components/responses/ErrorResponse'
        '404':
          $ref: '#/components/responses/ErrorResponse'
        '500':
          $ref: '#/components/responses/ErrorResponse'
        '503':
          $ref: '#/components/responses/ErrorResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    MarketCalendarResponse:
      type: object
      required:
        - market
        - marketCalendarId
        - kind
        - description
        - calendarUri
        - contentSha256
        - loadedAt
        - rawJson
      properties:
        calendar:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/CommodityMarketCalendarView'
        calendarUri:
          type: string
        contentSha256:
          type: string
        description:
          type: string
        kind:
          type: string
        loadedAt:
          type: string
          format: date-time
        market:
          type: string
        marketCalendarId:
          type: string
        rawJson:
          type: string
    CommodityMarketCalendarView:
      type: object
      required:
        - weeklySchedule
        - dateOverrides
      properties:
        dateOverrides:
          type: object
          additionalProperties:
            $ref: '#/components/schemas/CommodityMarketDaySchedule'
          propertyNames:
            type: string
        weeklySchedule:
          type: object
          additionalProperties:
            $ref: '#/components/schemas/CommodityMarketDaySchedule'
          propertyNames:
            type: string
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
    CommodityMarketDaySchedule:
      type: object
      required:
        - sessions
      properties:
        sessions:
          type: array
          items:
            $ref: '#/components/schemas/CommodityMarketSessionRange'
    CommodityMarketSessionRange:
      type: object
      description: A single named session within a [`CommodityMarketDaySchedule`].
      required:
        - start
        - end
        - mode
      properties:
        end:
          type: string
        mode:
          type: string
          description: Serialized calendar mode name, e.g. `"EXTERNAL"`, `"INTERNAL"`.
        start:
          type: string
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/exchange/get-market-calendar.md -->

---

# Source: `api/exchange/get-market-fills.md`

**Original URL:** `https://docs.phoenix.trade/api/exchange/get-market-fills.md`

**Local path:** `api/exchange/get-market-fills.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get market fills

> Returns unaggregated fill records for a market, ordered by time descending.
Only fills with valid transaction signatures are included.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/trades/{symbol}/fills
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/trades/{symbol}/fills:
    get:
      tags:
        - Exchange
      summary: Get market fills
      description: >-
        Returns unaggregated fill records for a market, ordered by time
        descending.

        Only fills with valid transaction signatures are included.
      operationId: get.v1.trades.by_symbol.fills
      parameters:
        - name: symbol
          in: path
          description: Market symbol
          required: true
          schema:
            type: string
          example: SOL
        - name: limit
          in: query
          description: 'Maximum number of trade records to return (default: 100, max: 10000)'
          required: false
          schema:
            type:
              - integer
              - 'null'
            format: int64
        - name: cursor
          in: query
          description: Optional opaque cursor returned by a previous request
          required: false
          schema:
            type:
              - string
              - 'null'
        - name: startTime
          in: query
          description: Inclusive start time in milliseconds since Unix epoch.
          required: false
          schema:
            type:
              - integer
              - 'null'
            format: int64
        - name: endTime
          in: query
          description: Exclusive end time in milliseconds since Unix epoch.
          required: false
          schema:
            type:
              - integer
              - 'null'
            format: int64
      responses:
        '200':
          description: Market fills retrieved successfully
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/FillsResponse'
        '400':
          $ref: '#/components/responses/ErrorResponse'
        '404':
          $ref: '#/components/responses/ErrorResponse'
        '500':
          $ref: '#/components/responses/ErrorResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    FillsResponse:
      type: object
      description: Fills response with pagination metadata.
      required:
        - data
        - hasMore
      properties:
        data:
          type: array
          items:
            $ref: '#/components/schemas/FillRecord'
        hasMore:
          type: boolean
        nextCursor:
          type:
            - string
            - 'null'
        prevCursor:
          type:
            - string
            - 'null'
    FillRecord:
      type: object
      description: Individual fill record returned by the fills channel.
      required:
        - marketSymbol
        - baseQty
        - quoteQty
        - price
        - timestamp
        - transactionSignature
        - instructionType
      properties:
        baseQty:
          type: string
          description: Human-readable base quantity.
        instructionType:
          type: string
          description: |-
            Instruction type that emitted this fill (e.g., PlaceMarketOrder,
            LiquidateViaMarketOrder).
        marketSymbol:
          type: string
          description: Market symbol associated with the fill (e.g., "SOL-PERP").
        price:
          type: string
          description: Human-readable price derived from the fill quantities.
        quoteQty:
          type: string
          description: Human-readable quote quantity.
        timestamp:
          type: string
          format: date-time
          description: Formatted datetime string (ISO 8601).
        transactionSignature:
          type: string
          description: Transaction signature containing the fill.
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/exchange/get-market-fills.md -->

---

# Source: `api/exchange/get-market-stats-history.md`

**Original URL:** `https://docs.phoenix.trade/api/exchange/get-market-stats-history.md`

**Local path:** `api/exchange/get-market-stats-history.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get market stats history

> Handles `GET /v1/market/{symbol}/stats` via `get.v1.market.by_symbol.stats`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/market/{symbol}/stats
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/market/{symbol}/stats:
    get:
      tags:
        - Exchange
      summary: Get market stats history
      description: >-
        Handles `GET /v1/market/{symbol}/stats` via
        `get.v1.market.by_symbol.stats`.
      operationId: get.v1.market.by_symbol.stats
      parameters:
        - name: symbol
          in: path
          description: Market symbol
          required: true
          schema:
            type: string
          example: SOL-PERP
        - name: start_time
          in: query
          description: Start time for market stats history (defaults to 1 hour ago)
          required: false
          schema:
            type:
              - string
              - 'null'
            format: date-time
          example: '2025-05-29T19:20:00.000Z'
        - name: end_time
          in: query
          description: End time for market stats history (defaults to now)
          required: false
          schema:
            type:
              - string
              - 'null'
            format: date-time
          example: '2025-05-29T19:25:00.000Z'
        - name: limit
          in: query
          description: Maximum number of data points to return (max 10000, default 1000)
          required: false
          schema:
            type:
              - integer
              - 'null'
            format: int64
          example: 1000
        - name: timeframe
          in: query
          description: >-
            Timeframe for aggregating stats (1s, 5s, 1m, 5m, 15m, 30m, 1h, 4h,
            1d)
          required: false
          schema:
            type:
              - string
              - 'null'
          example: 1h
      responses:
        '200':
          description: Successfully retrieved market stats history
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/MarketStatsHistoryResponse'
        '400':
          $ref: '#/components/responses/ErrorResponse'
        '404':
          $ref: '#/components/responses/ErrorResponse'
        '500':
          $ref: '#/components/responses/ErrorResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    MarketStatsHistoryResponse:
      type: object
      description: Response for market stats history endpoint
      required:
        - market_id
        - symbol
        - stats
      properties:
        market_id:
          type: integer
          format: int64
        stats:
          type: array
          items:
            $ref: '#/components/schemas/MarketStatsPoint'
        symbol:
          type: string
        timeframe:
          type:
            - string
            - 'null'
    MarketStatsPoint:
      type: object
      description: Individual market stats data point
      required:
        - open_interest
        - mark_price
        - spot_price
        - timestamp
        - slot
      properties:
        mark_price:
          type: number
          format: double
        open_interest:
          type: number
          format: double
        slot:
          type: integer
          format: int64
        spot_price:
          type: number
          format: double
        timestamp:
          type: string
          format: date-time
          description: Formatted datetime string (ISO 8601).
        total_maker_fees:
          type:
            - number
            - 'null'
          format: double
        total_taker_fees:
          type:
            - number
            - 'null'
          format: double
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/exchange/get-market-stats-history.md -->

---

# Source: `api/exchange/get-market.md`

**Original URL:** `https://docs.phoenix.trade/api/exchange/get-market.md`

**Local path:** `api/exchange/get-market.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get market

> Handles `GET /v1/view/exchange/market/{symbol}` via `get.v1.view.exchange.market.by_symbol`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/view/exchange/market/{symbol}
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/view/exchange/market/{symbol}:
    get:
      tags:
        - Exchange
      summary: Get market
      description: >-
        Handles `GET /v1/view/exchange/market/{symbol}` via
        `get.v1.view.exchange.market.by_symbol`.
      operationId: get.v1.view.exchange.market.by_symbol
      parameters:
        - name: symbol
          in: path
          description: Market symbol (e.g., SOL-PERP)
          required: true
          schema:
            type: string
      responses:
        '200':
          description: Market configuration
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ExchangeMarketConfig'
        '404':
          $ref: '#/components/responses/ErrorResponse'
        '500':
          $ref: '#/components/responses/ErrorResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    ExchangeMarketConfig:
      type: object
      description: >-
        Static market configuration without live data like prices or open
        interest.

        Used by the `/exchange` endpoint to return market parameters.
      required:
        - symbol
        - assetId
        - marketStatus
        - marketPubkey
        - splinePubkey
        - tickSize
        - baseLotsDecimals
        - takerFee
        - makerFee
        - leverageTiers
        - riskFactors
        - fundingIntervalSeconds
        - fundingPeriodSeconds
        - maxFundingRatePerInterval
        - openInterestCapBaseLots
        - maxLiquidationSizeBaseLots
        - isolatedOnly
      properties:
        assetId:
          type: integer
          format: int32
          description: Numeric asset identifier.
          minimum: 0
        baseLotsDecimals:
          type: integer
          format: int32
          description: Base-lot decimal exponent.
        commodityMetadata:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/CommodityMetadata'
              description: Commodity-specific metadata, present only for commodity markets.
        fundingIntervalSeconds:
          type: integer
          format: int32
          description: Funding interval length in seconds.
          minimum: 0
        fundingPeriodSeconds:
          type: integer
          format: int32
          description: Funding period length in seconds.
          minimum: 0
        isolatedOnly:
          type: boolean
          description: Whether this market only supports isolated margin positions
        leverageTiers:
          type: array
          items:
            $ref: '#/components/schemas/ExchangeLeverageTier'
          description: Configured leverage tiers.
        makerFee:
          type: number
          format: double
          description: Maker fee (percent).
        marketPubkey:
          type: string
          description: The orderbook account pubkey (base58 encoded)
        marketStatus:
          $ref: '#/components/schemas/MarketStatus'
          description: Current market status.
        maxFundingRatePerInterval:
          type: integer
          format: int64
          description: Maximum absolute funding rate per interval.
        maxFundingRatePerIntervalPercentage:
          type: number
          format: double
          description: >-
            Maximum absolute funding rate per interval as a percentage of
            notional

            at the current mark price.
        maxLiquidationSizeBaseLots:
          $ref: '#/components/schemas/JsSafeU64'
          description: Maximum size for a single liquidation (in base lots)
        metadata:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/MarketPublicMetadata'
              description: Public market metadata configured off-chain.
        openInterestCapBaseLots:
          $ref: '#/components/schemas/JsSafeU64'
          description: Maximum open interest allowed for this market (in base lots)
        riskFactors:
          $ref: '#/components/schemas/ExchangeRiskFactors'
          description: Risk factors as percentages.
        splinePubkey:
          type: string
          description: The spline collection PDA (derived from market_pubkey)
        statsSnapshot:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/MarketStatsSnapshot'
              description: >-
                Optional live stats snapshot for seeding clients before
                websocket

                updates.
        symbol:
          type: string
          description: Market symbol (for example, "SOL-PERP").
        takerFee:
          type: number
          format: double
          description: Taker fee (percent).
        tickSize:
          type: integer
          format: int64
          description: Tick size in quote lots per base lot per tick.
          minimum: 0
    CommodityMetadata:
      type: object
      description: Commodity-specific market metadata.
      required:
        - isCommodity
        - isReopen
        - isAfterHours
        - status
        - afterHoursRadius
      properties:
        afterHoursRadius:
          type: string
          description: Commodity after-hours radius in price units.
        executionPriceBand:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/MarketPriceBand'
              description: >-
                Price band currently applied to execution-visible orderbook
                levels.
        isAfterHours:
          type: boolean
          description: Parsed after-hours flag.
        isCommodity:
          type: boolean
          description: Parsed commodity flag. Always true when this object is present.
        isReopen:
          type: boolean
          description: Parsed reopen flag.
        lastIndexExpiryTimestamp:
          type:
            - integer
            - 'null'
          format: int64
          description: Unix timestamp when the last known index price expires.
          minimum: 0
        lastKnownIndexPrice:
          type:
            - string
            - 'null'
          description: Last index price used for commodity after-hours/reopen behavior.
        markPriceBand:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/MarketPriceBand'
              description: Mark-price clamp band around the last known index price.
        status:
          $ref: '#/components/schemas/CommodityMarketState'
          description: Derived commodity state.
    ExchangeLeverageTier:
      type: object
      description: |-
        Leverage tier with risk factors as f64 percentages.
        Used by the `/exchange` endpoint.
      required:
        - maxLeverage
        - maxSizeBaseLots
        - limitOrderRiskFactor
      properties:
        limitOrderRiskFactor:
          type: number
          format: double
          description: The limit order risk factor as a percentage (e.g., 60.0 = 60%).
        limitOrderRiskFactorBps:
          type:
            - integer
            - 'null'
          format: int32
          description: The limit order risk factor in basis points (e.g., 6000 = 60%).
          minimum: 0
        maxLeverage:
          type: number
          format: double
          description: Maximum leverage for this tier.
        maxSizeBaseLots:
          type: integer
          format: int64
          description: Maximum size in base lots for this tier.
          minimum: 0
    MarketStatus:
      type: string
      enum:
        - uninitialized
        - active
        - postOnly
        - paused
        - closed
        - tombstoned
    JsSafeU64:
      type: integer
      format: int64
      description: |-
        Wrapper for unsigned 64-bit values that must be JSON-safe for consumers
        written in JavaScript/TypeScript. Mirrors [`JsSafeI64`] but for unsigned
        Phoenix quantities such as base lots, quote lots, and slots.
      minimum: 0
    MarketPublicMetadata:
      type: object
      description: Public off-chain metadata for a market.
      properties:
        calendar:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/MarketCalendar'
              description: >-
                Market calendar metadata derived from the configured calendar
                id.
        coinGeckoId:
          type:
            - string
            - 'null'
          description: CoinGecko asset identifier.
        coinMarketCapId:
          type:
            - integer
            - 'null'
          format: int64
          description: CoinMarketCap numeric asset identifier.
        description:
          type:
            - string
            - 'null'
          description: Human-readable market description.
        displayColor:
          type:
            - string
            - 'null'
          description: Preferred display color for this market.
        logoUri:
          type:
            - string
            - 'null'
          description: Logo URI for this market.
        name:
          type:
            - string
            - 'null'
          description: Human-readable market name.
        tokensXyzAssetId:
          type:
            - string
            - 'null'
          description: tokens.xyz asset identifier.
    ExchangeRiskFactors:
      type: object
      description: |-
        Risk factors as percentages.
        Used by the `/exchange` endpoint.
      required:
        - maintenance
        - backstop
        - highRisk
        - upnl
        - upnlForWithdrawals
        - cancelOrder
      properties:
        backstop:
          type: number
          format: double
          description: Backstop liquidation risk factor as a percentage.
        backstopBps:
          type:
            - integer
            - 'null'
          format: int32
          description: Backstop liquidation risk factor in basis points.
          minimum: 0
        cancelOrder:
          type: number
          format: double
          description: Cancel order risk factor as a percentage.
        cancelOrderBps:
          type:
            - integer
            - 'null'
          format: int32
          description: Cancel order risk factor in basis points.
          minimum: 0
        highRisk:
          type: number
          format: double
          description: High risk threshold as a percentage.
        highRiskBps:
          type:
            - integer
            - 'null'
          format: int32
          description: High risk threshold in basis points.
          minimum: 0
        maintenance:
          type: number
          format: double
          description: Maintenance margin risk factor as a percentage (e.g., 50.0 = 50%).
        maintenanceBps:
          type:
            - integer
            - 'null'
          format: int32
          description: Maintenance margin risk factor in basis points (e.g., 5000 = 50%).
          minimum: 0
        upnl:
          type: number
          format: double
          description: Risk factor for positive unrealized PnL penalty as a percentage.
        upnlBps:
          type:
            - integer
            - 'null'
          format: int32
          description: Risk factor for positive unrealized PnL penalty in basis points.
          minimum: 0
        upnlForWithdrawals:
          type: number
          format: double
          description: >-
            Risk factor for positive unrealized PnL penalty during withdrawals
            as a

            percentage.
        upnlForWithdrawalsBps:
          type:
            - integer
            - 'null'
          format: int32
          description: >-
            Risk factor for positive unrealized PnL penalty during withdrawals
            in

            basis points.
          minimum: 0
    MarketStatsSnapshot:
      type: object
      description: Live market stats captured alongside an exchange market config response.
      required:
        - slot
        - slotIndex
        - openInterestBaseLots
        - fundingStartIntervalTimestamp
        - cumulativeFundingRate
      properties:
        cumulativeFundingRate:
          $ref: '#/components/schemas/JsSafeI64'
          description: Current cumulative funding rate.
        fundingStartIntervalTimestamp:
          $ref: '#/components/schemas/JsSafeU64'
          description: Unix timestamp in seconds for the current funding interval start.
        openInterestBaseLots:
          $ref: '#/components/schemas/JsSafeU64'
          description: Current open interest in base lots.
        slot:
          type: integer
          format: int64
          description: Solana slot of the state snapshot used to build these stats.
          minimum: 0
        slotIndex:
          type: integer
          format: int32
          description: |-
            Intra-slot sequence index of the state snapshot used to build these
            stats.
          minimum: 0
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
    MarketPriceBand:
      type: object
      description: Inclusive lower/upper price bounds for market-specific execution rules.
      required:
        - min
        - max
      properties:
        max:
          type: string
        min:
          type: string
    CommodityMarketState:
      type: string
      description: >-
        Represents the state of a RWA market. This type is shared between
        on-chain

        and off-chain components
      enum:
        - active
        - afterHours
        - reopen
    MarketCalendar:
      type: object
      description: Public metadata for a market calendar associated with a market.
      required:
        - id
        - description
        - calendarUri
        - contentSha256
      properties:
        calendarUri:
          type: string
          description: URI where the full market calendar can be fetched.
        contentSha256:
          type: string
          description: SHA-256 hash of the calendar content.
        description:
          type: string
          description: Human-readable calendar description.
        id:
          type: string
          description: Market calendar identifier configured off-chain.
        nextMarketTransitionUtc:
          type:
            - string
            - 'null'
          format: date-time
          description: Next UTC timestamp at which this calendar changes market state.
    JsSafeI64:
      type: integer
      format: int64
      description: >-
        Wrapper for signed 64-bit values that need to survive JSON transport
        without

        tripping JavaScript's safe-integer limits. When `serde` is enabled, this
        can

        deserialize from a string or number to accommodate clients that
        stringify

        large values.
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/exchange/get-market.md -->

---

# Source: `api/exchange/get-next-commodity-market-transition.md`

**Original URL:** `https://docs.phoenix.trade/api/exchange/get-next-commodity-market-transition.md`

**Local path:** `api/exchange/get-next-commodity-market-transition.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get next commodity market transition

> Handles `GET /v1/market/next-commodity-market-transition` via `get.v1.market.next_commodity_market_transition`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/market/next-commodity-market-transition
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/market/next-commodity-market-transition:
    get:
      tags:
        - Exchange
      summary: Get next commodity market transition
      description: >-
        Handles `GET /v1/market/next-commodity-market-transition` via
        `get.v1.market.next_commodity_market_transition`.
      operationId: get.v1.market.next_commodity_market_transition
      responses:
        '200':
          description: Next commodity market transition
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/NextCommodityMarketTransition'
        '400':
          $ref: '#/components/responses/ErrorResponse'
        '404':
          $ref: '#/components/responses/ErrorResponse'
        '500':
          $ref: '#/components/responses/ErrorResponse'
        '503':
          $ref: '#/components/responses/ErrorResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    NextCommodityMarketTransition:
      type: object
      required:
        - market
        - loadedAt
        - currentState
      properties:
        currentState:
          $ref: '#/components/schemas/CommodityMarketStateView'
        loadedAt:
          type: string
          format: date-time
        market:
          type: string
        nextMarketState:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/CommodityMarketStateView'
        utcNextTransition:
          type:
            - string
            - 'null'
          format: date-time
    CommodityMarketStateView:
      type: string
      enum:
        - open
        - afterHours
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/exchange/get-next-commodity-market-transition.md -->

---

# Source: `api/exchange/get-next-market-calendar-transition.md`

**Original URL:** `https://docs.phoenix.trade/api/exchange/get-next-market-calendar-transition.md`

**Local path:** `api/exchange/get-next-market-calendar-transition.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get next market calendar transition

> Handles `GET /v1/market/{symbol}/next-market-calendar-transition` via `get.v1.market.by_symbol.next_market_calendar_transition`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/market/{symbol}/next-market-calendar-transition
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/market/{symbol}/next-market-calendar-transition:
    get:
      tags:
        - Exchange
      summary: Get next market calendar transition
      description: >-
        Handles `GET /v1/market/{symbol}/next-market-calendar-transition` via
        `get.v1.market.by_symbol.next_market_calendar_transition`.
      operationId: get.v1.market.by_symbol.next_market_calendar_transition
      parameters:
        - name: symbol
          in: path
          description: Market symbol
          required: true
          schema:
            type: string
      responses:
        '200':
          description: Next market calendar transition
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/NextMarketCalendarTransition'
        '400':
          $ref: '#/components/responses/ErrorResponse'
        '404':
          $ref: '#/components/responses/ErrorResponse'
        '500':
          $ref: '#/components/responses/ErrorResponse'
        '503':
          $ref: '#/components/responses/ErrorResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    NextMarketCalendarTransition:
      type: object
      required:
        - market
        - marketCalendarId
        - calendarUri
        - loadedAt
        - currentState
      properties:
        calendarUri:
          type: string
        currentState:
          $ref: '#/components/schemas/MarketCalendarStateView'
        loadedAt:
          type: string
          format: date-time
        market:
          type: string
        marketCalendarId:
          type: string
        nextMarketState:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/MarketCalendarStateView'
        utcNextTransition:
          type:
            - string
            - 'null'
          format: date-time
    MarketCalendarStateView:
      type: string
      enum:
        - open
        - afterHours
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/exchange/get-next-market-calendar-transition.md -->

---

# Source: `api/exchange/get-orderbook.md`

**Original URL:** `https://docs.phoenix.trade/api/exchange/get-orderbook.md`

**Local path:** `api/exchange/get-orderbook.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get orderbook

> Handles `GET /v1/view/orderbook/{symbol}` via `get.v1.view.orderbook.by_symbol`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/view/orderbook/{symbol}
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/view/orderbook/{symbol}:
    get:
      tags:
        - Exchange
      summary: Get orderbook
      description: >-
        Handles `GET /v1/view/orderbook/{symbol}` via
        `get.v1.view.orderbook.by_symbol`.
      operationId: get.v1.view.orderbook.by_symbol
      parameters:
        - name: symbol
          in: path
          description: Trading symbol (BTC, ETH, SOL)
          required: true
          schema:
            type: string
        - name: include_splines
          in: query
          description: >-
            Whether to include splines in response and use them for parsed
            orderbook
          required: false
          schema:
            type: boolean
        - name: bypass_execution_band
          in: query
          description: >-
            Opt in to receive the full orderbook during commodities after-hours
            (no-op otherwise)
          required: false
          schema:
            type: boolean
      responses:
        '200':
          description: Phoenix Eternal orderbook
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/OrderbookView'
        '400':
          $ref: '#/components/responses/ErrorResponse'
        '404':
          $ref: '#/components/responses/ErrorResponse'
        '500':
          $ref: '#/components/responses/ErrorResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    OrderbookView:
      type: object
      description: View orderbook
      required:
        - slot
        - symbol
        - bids
        - asks
      properties:
        asks:
          type: array
          items:
            type: array
            items: false
            prefixItems:
              - type: number
                format: double
              - type: number
                format: double
          description: Ask levels as `(price, size)`.
        bids:
          type: array
          items:
            type: array
            items: false
            prefixItems:
              - type: number
                format: double
              - type: number
                format: double
          description: Bid levels as `(price, size)`.
        mid:
          type:
            - number
            - 'null'
          format: double
          description: Mid price, if both sides exist.
        slot:
          type: integer
          format: int64
          description: Solana slot of this orderbook snapshot.
          minimum: 0
        splines:
          type:
            - array
            - 'null'
          items:
            $ref: '#/components/schemas/Spline'
          description: Optional spline levels.
        symbol:
          type: string
          description: Market symbol.
    Spline:
      type: object
      required:
        - traderKey
        - midPrice
        - bidFilledAmount
        - askFilledAmount
        - bidRegions
        - askRegions
      properties:
        askFilledAmount:
          type: string
        askRegions:
          type: array
          items:
            $ref: '#/components/schemas/TickRegion'
        bidFilledAmount:
          type: string
        bidRegions:
          type: array
          items:
            $ref: '#/components/schemas/TickRegion'
        midPrice:
          type: number
          format: double
        traderKey:
          type: string
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
    TickRegion:
      type: object
      required:
        - start
        - end
        - density
        - totalSize
        - filledSize
      properties:
        density:
          type: number
          format: double
        end:
          type: number
          format: double
        filledSize:
          type: string
        start:
          type: number
          format: double
        totalSize:
          type: string
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/exchange/get-orderbook.md -->

---

# Source: `api/exchange/list-markets.md`

**Original URL:** `https://docs.phoenix.trade/api/exchange/list-markets.md`

**Local path:** `api/exchange/list-markets.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# List markets

> Handles `GET /v1/view/exchange/markets` via `get.v1.view.exchange.markets`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/view/exchange/markets
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/view/exchange/markets:
    get:
      tags:
        - Exchange
      summary: List markets
      description: >-
        Handles `GET /v1/view/exchange/markets` via
        `get.v1.view.exchange.markets`.
      operationId: get.v1.view.exchange.markets
      responses:
        '200':
          description: Exchange market configurations
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/ExchangeMarketConfig'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    ExchangeMarketConfig:
      type: object
      description: >-
        Static market configuration without live data like prices or open
        interest.

        Used by the `/exchange` endpoint to return market parameters.
      required:
        - symbol
        - assetId
        - marketStatus
        - marketPubkey
        - splinePubkey
        - tickSize
        - baseLotsDecimals
        - takerFee
        - makerFee
        - leverageTiers
        - riskFactors
        - fundingIntervalSeconds
        - fundingPeriodSeconds
        - maxFundingRatePerInterval
        - openInterestCapBaseLots
        - maxLiquidationSizeBaseLots
        - isolatedOnly
      properties:
        assetId:
          type: integer
          format: int32
          description: Numeric asset identifier.
          minimum: 0
        baseLotsDecimals:
          type: integer
          format: int32
          description: Base-lot decimal exponent.
        commodityMetadata:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/CommodityMetadata'
              description: Commodity-specific metadata, present only for commodity markets.
        fundingIntervalSeconds:
          type: integer
          format: int32
          description: Funding interval length in seconds.
          minimum: 0
        fundingPeriodSeconds:
          type: integer
          format: int32
          description: Funding period length in seconds.
          minimum: 0
        isolatedOnly:
          type: boolean
          description: Whether this market only supports isolated margin positions
        leverageTiers:
          type: array
          items:
            $ref: '#/components/schemas/ExchangeLeverageTier'
          description: Configured leverage tiers.
        makerFee:
          type: number
          format: double
          description: Maker fee (percent).
        marketPubkey:
          type: string
          description: The orderbook account pubkey (base58 encoded)
        marketStatus:
          $ref: '#/components/schemas/MarketStatus'
          description: Current market status.
        maxFundingRatePerInterval:
          type: integer
          format: int64
          description: Maximum absolute funding rate per interval.
        maxFundingRatePerIntervalPercentage:
          type: number
          format: double
          description: >-
            Maximum absolute funding rate per interval as a percentage of
            notional

            at the current mark price.
        maxLiquidationSizeBaseLots:
          $ref: '#/components/schemas/JsSafeU64'
          description: Maximum size for a single liquidation (in base lots)
        metadata:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/MarketPublicMetadata'
              description: Public market metadata configured off-chain.
        openInterestCapBaseLots:
          $ref: '#/components/schemas/JsSafeU64'
          description: Maximum open interest allowed for this market (in base lots)
        riskFactors:
          $ref: '#/components/schemas/ExchangeRiskFactors'
          description: Risk factors as percentages.
        splinePubkey:
          type: string
          description: The spline collection PDA (derived from market_pubkey)
        statsSnapshot:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/MarketStatsSnapshot'
              description: >-
                Optional live stats snapshot for seeding clients before
                websocket

                updates.
        symbol:
          type: string
          description: Market symbol (for example, "SOL-PERP").
        takerFee:
          type: number
          format: double
          description: Taker fee (percent).
        tickSize:
          type: integer
          format: int64
          description: Tick size in quote lots per base lot per tick.
          minimum: 0
    CommodityMetadata:
      type: object
      description: Commodity-specific market metadata.
      required:
        - isCommodity
        - isReopen
        - isAfterHours
        - status
        - afterHoursRadius
      properties:
        afterHoursRadius:
          type: string
          description: Commodity after-hours radius in price units.
        executionPriceBand:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/MarketPriceBand'
              description: >-
                Price band currently applied to execution-visible orderbook
                levels.
        isAfterHours:
          type: boolean
          description: Parsed after-hours flag.
        isCommodity:
          type: boolean
          description: Parsed commodity flag. Always true when this object is present.
        isReopen:
          type: boolean
          description: Parsed reopen flag.
        lastIndexExpiryTimestamp:
          type:
            - integer
            - 'null'
          format: int64
          description: Unix timestamp when the last known index price expires.
          minimum: 0
        lastKnownIndexPrice:
          type:
            - string
            - 'null'
          description: Last index price used for commodity after-hours/reopen behavior.
        markPriceBand:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/MarketPriceBand'
              description: Mark-price clamp band around the last known index price.
        status:
          $ref: '#/components/schemas/CommodityMarketState'
          description: Derived commodity state.
    ExchangeLeverageTier:
      type: object
      description: |-
        Leverage tier with risk factors as f64 percentages.
        Used by the `/exchange` endpoint.
      required:
        - maxLeverage
        - maxSizeBaseLots
        - limitOrderRiskFactor
      properties:
        limitOrderRiskFactor:
          type: number
          format: double
          description: The limit order risk factor as a percentage (e.g., 60.0 = 60%).
        limitOrderRiskFactorBps:
          type:
            - integer
            - 'null'
          format: int32
          description: The limit order risk factor in basis points (e.g., 6000 = 60%).
          minimum: 0
        maxLeverage:
          type: number
          format: double
          description: Maximum leverage for this tier.
        maxSizeBaseLots:
          type: integer
          format: int64
          description: Maximum size in base lots for this tier.
          minimum: 0
    MarketStatus:
      type: string
      enum:
        - uninitialized
        - active
        - postOnly
        - paused
        - closed
        - tombstoned
    JsSafeU64:
      type: integer
      format: int64
      description: |-
        Wrapper for unsigned 64-bit values that must be JSON-safe for consumers
        written in JavaScript/TypeScript. Mirrors [`JsSafeI64`] but for unsigned
        Phoenix quantities such as base lots, quote lots, and slots.
      minimum: 0
    MarketPublicMetadata:
      type: object
      description: Public off-chain metadata for a market.
      properties:
        calendar:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/MarketCalendar'
              description: >-
                Market calendar metadata derived from the configured calendar
                id.
        coinGeckoId:
          type:
            - string
            - 'null'
          description: CoinGecko asset identifier.
        coinMarketCapId:
          type:
            - integer
            - 'null'
          format: int64
          description: CoinMarketCap numeric asset identifier.
        description:
          type:
            - string
            - 'null'
          description: Human-readable market description.
        displayColor:
          type:
            - string
            - 'null'
          description: Preferred display color for this market.
        logoUri:
          type:
            - string
            - 'null'
          description: Logo URI for this market.
        name:
          type:
            - string
            - 'null'
          description: Human-readable market name.
        tokensXyzAssetId:
          type:
            - string
            - 'null'
          description: tokens.xyz asset identifier.
    ExchangeRiskFactors:
      type: object
      description: |-
        Risk factors as percentages.
        Used by the `/exchange` endpoint.
      required:
        - maintenance
        - backstop
        - highRisk
        - upnl
        - upnlForWithdrawals
        - cancelOrder
      properties:
        backstop:
          type: number
          format: double
          description: Backstop liquidation risk factor as a percentage.
        backstopBps:
          type:
            - integer
            - 'null'
          format: int32
          description: Backstop liquidation risk factor in basis points.
          minimum: 0
        cancelOrder:
          type: number
          format: double
          description: Cancel order risk factor as a percentage.
        cancelOrderBps:
          type:
            - integer
            - 'null'
          format: int32
          description: Cancel order risk factor in basis points.
          minimum: 0
        highRisk:
          type: number
          format: double
          description: High risk threshold as a percentage.
        highRiskBps:
          type:
            - integer
            - 'null'
          format: int32
          description: High risk threshold in basis points.
          minimum: 0
        maintenance:
          type: number
          format: double
          description: Maintenance margin risk factor as a percentage (e.g., 50.0 = 50%).
        maintenanceBps:
          type:
            - integer
            - 'null'
          format: int32
          description: Maintenance margin risk factor in basis points (e.g., 5000 = 50%).
          minimum: 0
        upnl:
          type: number
          format: double
          description: Risk factor for positive unrealized PnL penalty as a percentage.
        upnlBps:
          type:
            - integer
            - 'null'
          format: int32
          description: Risk factor for positive unrealized PnL penalty in basis points.
          minimum: 0
        upnlForWithdrawals:
          type: number
          format: double
          description: >-
            Risk factor for positive unrealized PnL penalty during withdrawals
            as a

            percentage.
        upnlForWithdrawalsBps:
          type:
            - integer
            - 'null'
          format: int32
          description: >-
            Risk factor for positive unrealized PnL penalty during withdrawals
            in

            basis points.
          minimum: 0
    MarketStatsSnapshot:
      type: object
      description: Live market stats captured alongside an exchange market config response.
      required:
        - slot
        - slotIndex
        - openInterestBaseLots
        - fundingStartIntervalTimestamp
        - cumulativeFundingRate
      properties:
        cumulativeFundingRate:
          $ref: '#/components/schemas/JsSafeI64'
          description: Current cumulative funding rate.
        fundingStartIntervalTimestamp:
          $ref: '#/components/schemas/JsSafeU64'
          description: Unix timestamp in seconds for the current funding interval start.
        openInterestBaseLots:
          $ref: '#/components/schemas/JsSafeU64'
          description: Current open interest in base lots.
        slot:
          type: integer
          format: int64
          description: Solana slot of the state snapshot used to build these stats.
          minimum: 0
        slotIndex:
          type: integer
          format: int32
          description: |-
            Intra-slot sequence index of the state snapshot used to build these
            stats.
          minimum: 0
    MarketPriceBand:
      type: object
      description: Inclusive lower/upper price bounds for market-specific execution rules.
      required:
        - min
        - max
      properties:
        max:
          type: string
        min:
          type: string
    CommodityMarketState:
      type: string
      description: >-
        Represents the state of a RWA market. This type is shared between
        on-chain

        and off-chain components
      enum:
        - active
        - afterHours
        - reopen
    MarketCalendar:
      type: object
      description: Public metadata for a market calendar associated with a market.
      required:
        - id
        - description
        - calendarUri
        - contentSha256
      properties:
        calendarUri:
          type: string
          description: URI where the full market calendar can be fetched.
        contentSha256:
          type: string
          description: SHA-256 hash of the calendar content.
        description:
          type: string
          description: Human-readable calendar description.
        id:
          type: string
          description: Market calendar identifier configured off-chain.
        nextMarketTransitionUtc:
          type:
            - string
            - 'null'
          format: date-time
          description: Next UTC timestamp at which this calendar changes market state.
    JsSafeI64:
      type: integer
      format: int64
      description: >-
        Wrapper for signed 64-bit values that need to survive JSON transport
        without

        tripping JavaScript's safe-integer limits. When `serde` is enabled, this
        can

        deserialize from a string or number to accommodate clients that
        stringify

        large values.
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/exchange/list-markets.md -->

---

# Source: `api/exchange/submit-register-trader-transaction.md`

**Original URL:** `https://docs.phoenix.trade/api/exchange/submit-register-trader-transaction.md`

**Local path:** `api/exchange/submit-register-trader-transaction.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Submit register trader transaction

> Validates a base64-encoded transaction for the default trader account, signs it with the Phoenix onboarder, simulates it to ensure the onboarder pays no lamports, sends it to Phoenix's RPC, and returns the transaction signature.



## OpenAPI

````yaml /openapi/phoenix-public-api.json post /v1/exchange/send-register-ixs
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/exchange/send-register-ixs:
    post:
      tags:
        - Exchange
      summary: Submit register trader transaction
      description: >-
        Validates a base64-encoded transaction for the default trader account,
        signs it with the Phoenix onboarder, simulates it to ensure the
        onboarder pays no lamports, sends it to Phoenix's RPC, and returns the
        transaction signature.
      operationId: post.v1.exchange.send_register_ixs
      requestBody:
        description: JSON request payload for `post.v1.exchange.send_register_ixs`.
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/SendRegisterIxsRequest'
            example:
              traderAuthority: string
              transaction: string
              txFeePayer: string
        required: true
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/SendRegisterIxsResponse'
components:
  schemas:
    SendRegisterIxsRequest:
      type: object
      description: Request payload for /exchange/send-register-ixs.
      required:
        - transaction
        - traderAuthority
        - txFeePayer
      properties:
        maxPositions:
          type:
            - integer
            - 'null'
          format: int32
          minimum: 0
        traderAuthority:
          type: string
        traderPdaIndex:
          type:
            - integer
            - 'null'
          format: int32
          minimum: 0
        traderSubaccountIndex:
          type:
            - integer
            - 'null'
          format: int32
          minimum: 0
        transaction:
          type: string
          description: >-
            Base64-encoded Solana transaction wire bytes signed by the
            user-owned

            required signers. The API validates, signs as onboarder, simulates,
            and

            submits it.
        txFeePayer:
          type: string
    SendRegisterIxsResponse:
      type: object
      description: Response payload for /exchange/send-register-ixs.
      required:
        - signature
        - traderPda
        - traderOnboarder
        - txFeePayer
        - maxPositions
        - includeRegisterTrader
      properties:
        includeRegisterTrader:
          type: boolean
        maxPositions:
          type: integer
          format: int32
          minimum: 0
        signature:
          type: string
        traderOnboarder:
          type: string
        traderPda:
          type: string
        txFeePayer:
          type: string

````

<!-- END SOURCE: api/exchange/submit-register-trader-transaction.md -->

---

# Source: `api/index.md`

**Original URL:** `https://docs.phoenix.trade/api/index.md`

**Local path:** `api/index.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# API

> Phoenix REST and WebSocket API endpoints for market data, trader state, and transaction building.

## API Overview

Phoenix exposes a public REST API for snapshots and request/response workflows, plus a WebSocket API for live subscriptions.

| Surface   | URL                                  |
| --------- | ------------------------------------ |
| REST API  | `https://perp-api.phoenix.trade`     |
| WebSocket | `wss://perp-api.phoenix.trade/v1/ws` |

Use REST when you need a point-in-time response, such as exchange configuration, market metadata, trader state, historical fills, or transaction-builder responses. Use WebSocket subscriptions when you need continuous updates, such as orderbooks, trades, candles, trader state, or exchange parameter changes.

## REST API

The REST API accepts and returns JSON. Requests with bodies should send `Content-Type: application/json`.

The public reference is organized into:

* `Auth` — wallet, service, and session authentication.
* `Exchange` — exchange, market, candle, and no-referral register-instruction workflows.
* `Invite` — invite validation and referral activation, including user-funded referral activation transactions.
* `Trader` — trader state, account history, and transaction builders.

Some numeric values are string-encoded in responses so clients can preserve full integer precision. Use the schema in the REST API reference for exact field types.

### Authentication and Errors

Most exchange, market, and trader read endpoints are public. Routes that require a session use bearer tokens returned by the authentication endpoints; the REST API reference marks those requirements on each operation.

Error responses use a JSON object with an `error` string:

```json theme={null}
{
  "error": "error_code_or_message"
}
```

## WebSocket API

The WebSocket API uses JSON client messages:

```json theme={null}
{
  "type": "subscribe",
  "subscription": {
    "channel": "orderbook",
    "symbol": "SOL"
  }
}
```

The WebSocket protocol page documents supported channels, request envelopes, confirmation/error messages, and response payloads.

<!-- END SOURCE: api/index.md -->

---

# Source: `api/invite/activate-referral-deprecated-june-30-2026.md`

**Original URL:** `https://docs.phoenix.trade/api/invite/activate-referral-deprecated-june-30-2026.md`

**Local path:** `api/invite/activate-referral-deprecated-june-30-2026.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Activate referral (deprecated June 30, 2026)

> Deprecated on June 30, 2026. New onboarding should use delegated on-chain permission accounts or `/v1/referral/activate-tx` instead.



## OpenAPI

````yaml /openapi/phoenix-public-api.json post /v1/referral/activate
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/referral/activate:
    post:
      tags:
        - Invite
      summary: Activate referral (deprecated June 30, 2026)
      description: >-
        Deprecated on June 30, 2026. New onboarding should use delegated
        on-chain permission accounts or `/v1/referral/activate-tx` instead.
      operationId: post.v1.referral.activate
      requestBody:
        description: JSON request payload for `post.v1.referral.activate`.
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/ActivateInviteWithReferralRequest'
            example:
              authority: string
              referral_code: string
        required: true
      responses:
        '200':
          description: Trader created
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ActivateInviteResponse'
        '400':
          $ref: '#/components/responses/ErrorResponse'
        '401':
          $ref: '#/components/responses/ErrorResponse'
        '403':
          $ref: '#/components/responses/ErrorResponse'
        '409':
          $ref: '#/components/responses/ErrorResponse'
        '429':
          $ref: '#/components/responses/ErrorResponse'
        '504':
          $ref: '#/components/responses/ErrorResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    ActivateInviteWithReferralRequest:
      type: object
      required:
        - authority
        - referral_code
      properties:
        authority:
          type: string
        referral_code:
          type: string
    ActivateInviteResponse:
      type: object
      required:
        - trader_pda
      properties:
        trader_pda:
          type: string
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/invite/activate-referral-deprecated-june-30-2026.md -->

---

# Source: `api/invite/activate-referral-with-user-funded-transaction.md`

**Original URL:** `https://docs.phoenix.trade/api/invite/activate-referral-with-user-funded-transaction.md`

**Local path:** `api/invite/activate-referral-with-user-funded-transaction.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Activate referral with user-funded transaction

> Activates a trader with a valid referral code by validating a user-funded transaction, signing it with the referral activation onboarder, and submitting it. The trader authority or another non-Phoenix payer must pay fees and trader-account rent.



## OpenAPI

````yaml /openapi/phoenix-public-api.json post /v1/referral/activate-tx
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/referral/activate-tx:
    post:
      tags:
        - Invite
      summary: Activate referral with user-funded transaction
      description: >-
        Activates a trader with a valid referral code by validating a
        user-funded transaction, signing it with the referral activation
        onboarder, and submitting it. The trader authority or another
        non-Phoenix payer must pay fees and trader-account rent.
      operationId: post.v1.referral.activate_tx
      requestBody:
        description: JSON request payload for `post.v1.referral.activate_tx`.
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/ActivateReferralTxRequest'
            example:
              recent_blockhash: string
              referral_code: string
              trader_authority: string
              transaction: string
        required: true
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ActivateReferralTxResponse'
components:
  schemas:
    ActivateReferralTxRequest:
      type: object
      required:
        - referral_code
        - trader_authority
        - recent_blockhash
        - transaction
      properties:
        recent_blockhash:
          type: string
        referral_code:
          type: string
        trader_authority:
          type: string
        trader_pda_index:
          type:
            - integer
            - 'null'
          format: int32
          minimum: 0
        trader_subaccount_index:
          type:
            - integer
            - 'null'
          format: int32
          minimum: 0
        transaction:
          type: string
    ActivateReferralTxResponse:
      type: object
      required:
        - trader_pda
        - referral_code
        - status
      properties:
        referral_code:
          type: string
        signature:
          type:
            - string
            - 'null'
        status:
          $ref: '#/components/schemas/ActivateReferralTxStatus'
        trader_pda:
          type: string
    ActivateReferralTxStatus:
      type: string
      enum:
        - activated
        - submitted
        - already_activated

````

<!-- END SOURCE: api/invite/activate-referral-with-user-funded-transaction.md -->

---

# Source: `api/invite/check-wallet-invite-status.md`

**Original URL:** `https://docs.phoenix.trade/api/invite/check-wallet-invite-status.md`

**Local path:** `api/invite/check-wallet-invite-status.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Check wallet invite status

> Checks whether a wallet address is whitelisted and returns associated information.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/invite/check/{wallet}
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/invite/check/{wallet}:
    get:
      tags:
        - Invite
      summary: Check wallet invite status
      description: >-
        Checks whether a wallet address is whitelisted and returns associated
        information.
      operationId: get.v1.invite.check.by_wallet
      parameters:
        - name: wallet
          in: path
          description: Solana wallet address to check
          required: true
          schema:
            type: string
      responses:
        '200':
          description: Whitelist status retrieved
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/CheckWalletResponse'
        '400':
          $ref: '#/components/responses/ErrorResponse'
        '500':
          $ref: '#/components/responses/ErrorResponse'
        '503':
          $ref: '#/components/responses/ErrorResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    CheckWalletResponse:
      type: object
      required:
        - whitelisted
      properties:
        invite_code_used:
          type:
            - string
            - 'null'
        whitelisted:
          type: boolean
        whitelisted_at:
          type:
            - string
            - 'null'
          format: date-time
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/invite/check-wallet-invite-status.md -->

---

# Source: `api/invite/get-referral-activation-permission.md`

**Original URL:** `https://docs.phoenix.trade/api/invite/get-referral-activation-permission.md`

**Local path:** `api/invite/get-referral-activation-permission.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get referral activation permission

> Returns the trader onboarder, risk authority, and permission account needed to build a referral activation transaction.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/referral/activation-permission
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/referral/activation-permission:
    get:
      tags:
        - Invite
      summary: Get referral activation permission
      description: >-
        Returns the trader onboarder, risk authority, and permission account
        needed to build a referral activation transaction.
      operationId: get.v1.referral.activation_permission
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ReferralActivationPermissionResponse'
components:
  schemas:
    ReferralActivationPermissionResponse:
      type: object
      required:
        - trader_onboarder
        - risk_authority
        - permission_account
      properties:
        permission_account:
          type: string
        risk_authority:
          type: string
        trader_onboarder:
          type: string

````

<!-- END SOURCE: api/invite/get-referral-activation-permission.md -->

---

# Source: `api/invite/validate-invite.md`

**Original URL:** `https://docs.phoenix.trade/api/invite/validate-invite.md`

**Local path:** `api/invite/validate-invite.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Validate invite

> Validates an invite code and whitelists the wallet if the code is valid and unused.



## OpenAPI

````yaml /openapi/phoenix-public-api.json post /v1/invite/validate
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/invite/validate:
    post:
      tags:
        - Invite
      summary: Validate invite
      description: >-
        Validates an invite code and whitelists the wallet if the code is valid
        and unused.
      operationId: post.v1.invite.validate
      requestBody:
        description: JSON request payload for `post.v1.invite.validate`.
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/ValidateInviteRequest'
            example:
              code: string
              wallet_address: string
        required: true
      responses:
        '200':
          description: Validation successful
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ValidateInviteResponse'
        '400':
          $ref: '#/components/responses/ErrorResponse'
        '409':
          $ref: '#/components/responses/ErrorResponse'
        '429':
          $ref: '#/components/responses/ErrorResponse'
        '500':
          $ref: '#/components/responses/ErrorResponse'
        '503':
          $ref: '#/components/responses/ErrorResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    ValidateInviteRequest:
      type: object
      required:
        - code
        - wallet_address
      properties:
        code:
          type: string
        transaction_signature:
          type:
            - string
            - 'null'
          description: >-
            Optional transaction signature if the client has already submitted
            an

            on-chain transaction
        wallet_address:
          type: string
    ValidateInviteResponse:
      type: object
      required:
        - success
        - message
        - whitelisted
      properties:
        message:
          type: string
        success:
          type: boolean
        whitelisted:
          type: boolean
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/invite/validate-invite.md -->

---

# Source: `api/notifications/acknowledge-notifications-up-to-timestamp.md`

**Original URL:** `https://docs.phoenix.trade/api/notifications/acknowledge-notifications-up-to-timestamp.md`

**Local path:** `api/notifications/acknowledge-notifications-up-to-timestamp.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Acknowledge notifications up to timestamp

> Handles `POST /v1/traders/{trader_pubkey}/notifications/ack/up-to` via `post.v1.traders.by_trader_pubkey.notifications.ack.up_to`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json post /v1/traders/{trader_pubkey}/notifications/ack/up-to
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/traders/{trader_pubkey}/notifications/ack/up-to:
    post:
      tags:
        - Notifications
      summary: Acknowledge notifications up to timestamp
      description: >-
        Handles `POST /v1/traders/{trader_pubkey}/notifications/ack/up-to` via
        `post.v1.traders.by_trader_pubkey.notifications.ack.up_to`.
      operationId: post.v1.traders.by_trader_pubkey.notifications.ack.up_to
      parameters:
        - name: trader_pubkey
          in: path
          description: Trader pubkey (base58)
          required: true
          schema:
            type: string
      requestBody:
        description: >-
          JSON request payload for
          `post.v1.traders.by_trader_pubkey.notifications.ack.up_to`.
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/AckBeforeTimestampBody'
            example:
              beforeTimestamp: string
        required: true
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema: {}
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    AckBeforeTimestampBody:
      type: object
      required:
        - beforeTimestamp
      properties:
        beforeTimestamp:
          type: string
          format: date-time
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/notifications/acknowledge-notifications-up-to-timestamp.md -->

---

# Source: `api/notifications/acknowledge-notifications.md`

**Original URL:** `https://docs.phoenix.trade/api/notifications/acknowledge-notifications.md`

**Local path:** `api/notifications/acknowledge-notifications.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Acknowledge notifications

> Handles `POST /v1/traders/{trader_pubkey}/notifications/ack/notifications` via `post.v1.traders.by_trader_pubkey.notifications.ack.notifications`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json post /v1/traders/{trader_pubkey}/notifications/ack/notifications
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/traders/{trader_pubkey}/notifications/ack/notifications:
    post:
      tags:
        - Notifications
      summary: Acknowledge notifications
      description: >-
        Handles `POST
        /v1/traders/{trader_pubkey}/notifications/ack/notifications` via
        `post.v1.traders.by_trader_pubkey.notifications.ack.notifications`.
      operationId: post.v1.traders.by_trader_pubkey.notifications.ack.notifications
      parameters:
        - name: trader_pubkey
          in: path
          description: Trader pubkey (base58)
          required: true
          schema:
            type: string
      requestBody:
        description: >-
          JSON request payload for
          `post.v1.traders.by_trader_pubkey.notifications.ack.notifications`.
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/AckNotificationsBody'
            example:
              items:
                - type: event
        required: true
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema: {}
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    AckNotificationsBody:
      type: object
      required:
        - items
      properties:
        items:
          type: array
          items:
            $ref: '#/components/schemas/AckNotificationItem'
    AckNotificationItem:
      oneOf:
        - type: object
          required:
            - type
          properties:
            eventIndex:
              type:
                - integer
                - 'null'
              format: int32
            id:
              type:
                - integer
                - 'null'
              format: int64
            instructionIndex:
              type:
                - integer
                - 'null'
              format: int32
            recipientIndex:
              type:
                - integer
                - 'null'
              format: int32
            slot:
              type:
                - integer
                - 'null'
              format: int64
            slotIndex:
              type:
                - integer
                - 'null'
              format: int32
            type:
              type: string
              enum:
                - event
        - type: object
          required:
            - id
            - type
          properties:
            id:
              type: integer
              format: int64
            type:
              type: string
              enum:
                - admin
        - type: object
          required:
            - id
            - type
          properties:
            id:
              type: integer
              format: int64
            type:
              type: string
              enum:
                - general
      description: >-
        Event ack: by DB id or by composite index (slot, slotIndex,

        instructionIndex, eventIndex, recipientIndex). JSON uses camelCase for

        field names and lowercase for the "type" discriminant ("event" | "admin"
        |

        "general").
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/notifications/acknowledge-notifications.md -->

---

# Source: `api/notifications/list-trader-notifications.md`

**Original URL:** `https://docs.phoenix.trade/api/notifications/list-trader-notifications.md`

**Local path:** `api/notifications/list-trader-notifications.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# List trader notifications

> Handles `GET /v1/traders/{trader_pubkey}/notifications` via `get.v1.traders.by_trader_pubkey.notifications`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/traders/{trader_pubkey}/notifications
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/traders/{trader_pubkey}/notifications:
    get:
      tags:
        - Notifications
      summary: List trader notifications
      description: >-
        Handles `GET /v1/traders/{trader_pubkey}/notifications` via
        `get.v1.traders.by_trader_pubkey.notifications`.
      operationId: get.v1.traders.by_trader_pubkey.notifications
      parameters:
        - name: trader_pubkey
          in: path
          description: Trader pubkey (base58)
          required: true
          schema:
            type: string
        - name: limit
          in: query
          required: false
          schema:
            type:
              - integer
              - 'null'
            format: int64
        - name: cursor
          in: query
          required: false
          schema:
            type:
              - string
              - 'null'
        - name: unackedOnly
          in: query
          description: >-
            When true, return only unacked notifications with a fixed cap of
            1000

            (cursor and limit are ignored).
          required: false
          schema:
            type:
              - boolean
              - 'null'
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/GetNotificationsResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    GetNotificationsResponse:
      type: object
      required:
        - items
      properties:
        items:
          type: array
          items:
            $ref: '#/components/schemas/NotificationItem'
        nextCursor:
          type:
            - string
            - 'null'
    NotificationItem:
      oneOf:
        - type: object
          required:
            - id
            - slot
            - slotIndex
            - instructionIndex
            - eventIndex
            - recipientIndex
            - notificationType
            - data
            - createdAt
            - acked
            - source
          properties:
            acked:
              type: boolean
            createdAt:
              type: string
              format: date-time
            data: {}
            details:
              oneOf:
                - type: 'null'
                - $ref: '#/components/schemas/NotificationFromEventDetails'
                  description: >-
                    Structured details per notification type. None for legacy
                    rows.
            eventIndex:
              type: integer
              format: int32
            id:
              type: integer
              format: int64
            instructionIndex:
              type: integer
              format: int32
            notificationType:
              type: string
            recipientIndex:
              type: integer
              format: int32
            slot:
              type: integer
              format: int64
              description: |-
                Composite index for dedupe/ack; matches DB unique key and WS
                temporary id.
            slotIndex:
              type: integer
              format: int32
            source:
              type: string
              enum:
                - event
        - type: object
          required:
            - id
            - notificationType
            - data
            - createdAt
            - acked
            - source
          properties:
            acked:
              type: boolean
            body:
              type:
                - string
                - 'null'
            createdAt:
              type: string
              format: date-time
            data: {}
            id:
              type: integer
              format: int64
            notificationType:
              type: string
            source:
              type: string
              enum:
                - admin
            title:
              type:
                - string
                - 'null'
        - type: object
          description: >-
            General notification: user-specific, real-time (e.g. from Redis),
            not

            from on-chain events nor global admin announcements.
          required:
            - id
            - notificationType
            - data
            - createdAt
            - acked
            - source
          properties:
            acked:
              type: boolean
            body:
              type:
                - string
                - 'null'
            createdAt:
              type: string
              format: date-time
            data: {}
            id:
              type: integer
              format: int64
            notificationType:
              type: string
            source:
              type: string
              enum:
                - general
            title:
              type:
                - string
                - 'null'
      description: |-
        Notification item: discriminated union by source (event vs admin vs
        general).
    NotificationFromEventDetails:
      oneOf:
        - type: object
          required:
            - symbol
            - side
            - base_lots_filled
            - price
            - type
          properties:
            base_lots_filled:
              type: integer
              format: int64
            price:
              type: number
              format: double
            side:
              type: string
            symbol:
              type: string
            type:
              type: string
              enum:
                - orderFilled
        - type: object
          required:
            - symbol
            - side
            - base_lots_released
            - type
          properties:
            base_lots_released:
              type: integer
              format: int64
            side:
              type: string
            symbol:
              type: string
            type:
              type: string
              enum:
                - riskEngineCancelOrder
        - type: object
          required:
            - symbol
            - side
            - base_amount
            - quote_amount
            - type
          properties:
            base_amount:
              type: number
              format: double
            quote_amount:
              type: number
              format: double
            side:
              type: string
            symbol:
              type: string
            trigger_type:
              type:
                - string
                - 'null'
            type:
              type: string
              enum:
                - stopLossExecuted
        - type: object
          required:
            - symbol
            - side
            - base_amount
            - quote_amount
            - type
          properties:
            base_amount:
              type: number
              format: double
            quote_amount:
              type: number
              format: double
            side:
              type: string
            symbol:
              type: string
            trigger_type:
              type:
                - string
                - 'null'
            type:
              type: string
              enum:
                - conditionalOrderExecuted
        - type: object
          required:
            - symbol
            - side
            - base_amount
            - quote_amount
            - type
          properties:
            base_amount:
              type: number
              format: double
            quote_amount:
              type: number
              format: double
            side:
              type: string
            symbol:
              type: string
            type:
              type: string
              enum:
                - liquidation
        - type: object
          required:
            - asset_id
            - base_lots_closed
            - type
          properties:
            asset_id:
              type: integer
              format: int64
              minimum: 0
            base_lots_closed:
              type: integer
              format: int64
            type:
              type: string
              enum:
                - adl
        - type: object
          required:
            - asset_id
            - base_lots_transferred
            - virtual_quote_lots_transferred
            - haircut_rate
            - type
          properties:
            asset_id:
              type: integer
              format: int64
              minimum: 0
            base_lots_transferred:
              type: integer
              format: int64
            haircut_rate:
              type: integer
              format: int32
              minimum: 0
            type:
              type: string
              enum:
                - backstopLiquidation
            virtual_quote_lots_transferred:
              type: integer
              format: int64
        - type: object
          description: DB fallback for old rows. Never serialized to the API response.
          required:
            - type
          properties:
            type:
              type: string
              enum:
                - unknown
      description: >-
        Structured details for each notification type. Serialized to JSON for
        the

        `details` DB column and API response field.
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/notifications/list-trader-notifications.md -->

---

# Source: `api/trader/build-attached-conditional-order-transaction.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/build-attached-conditional-order-transaction.md`

**Local path:** `api/trader/build-attached-conditional-order-transaction.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Build attached conditional order transaction

> Handles `POST /v1/ix/place-attached-conditional-order` via `post.v1.ix.place_attached_conditional_order`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json post /v1/ix/place-attached-conditional-order
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/ix/place-attached-conditional-order:
    post:
      tags:
        - Trader
      summary: Build attached conditional order transaction
      description: >-
        Handles `POST /v1/ix/place-attached-conditional-order` via
        `post.v1.ix.place_attached_conditional_order`.
      operationId: post.v1.ix.place_attached_conditional_order
      requestBody:
        description: >-
          JSON request payload for
          `post.v1.ix.place_attached_conditional_order`.
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/PlaceAttachedConditionalOrderRequest'
            example:
              authority: string
              orderPriceInTicks: 0
              orderSequenceNumber: string
              symbol: string
              traderPdaIndex: 0
        required: true
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/ApiInstructionResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    PlaceAttachedConditionalOrderRequest:
      type: object
      description: Request payload for /v1/ix/place-attached-conditional-order.
      required:
        - authority
        - traderPdaIndex
        - symbol
        - orderSequenceNumber
        - orderPriceInTicks
      properties:
        authority:
          type: string
        feePayer:
          type:
            - string
            - 'null'
        flightBuilderAuthority:
          type:
            - string
            - 'null'
        flightFeeCollectorTrader:
          type:
            - string
            - 'null'
        greaterTrigger:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/ConditionalTriggerRequest'
        isIsolated:
          type: boolean
        lessTrigger:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/ConditionalTriggerRequest'
        orderPriceInTicks:
          type: integer
          format: int64
          minimum: 0
        orderSequenceNumber:
          type: string
        positionAuthority:
          type:
            - string
            - 'null'
        symbol:
          type: string
        traderPdaIndex:
          type: integer
          format: int32
          minimum: 0
        traderSubaccountIndex:
          type:
            - integer
            - 'null'
          format: int32
          minimum: 0
    ApiInstructionResponse:
      type: object
      description: |-
        API representation of a Solana instruction.

        Note: this is a pure DTO. When needed, clients/servers can enable the
        appropriate conversion logic in the consumer crate to avoid forcing a
        Solana dependency in `phoenix-api-types`.
      required:
        - data
        - keys
        - programId
      properties:
        data:
          type: array
          items:
            type: integer
            format: int32
            minimum: 0
        keys:
          type: array
          items:
            $ref: '#/components/schemas/ApiAccountMeta'
        programId:
          type: string
    ConditionalTriggerRequest:
      type: object
      description: Trigger configuration for attached conditional orders.
      required:
        - side
      properties:
        executionPrice:
          type:
            - number
            - 'null'
          format: double
        executionPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        orderKind:
          type:
            - string
            - 'null'
        side:
          type: string
        triggerPrice:
          type:
            - number
            - 'null'
          format: double
        triggerPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
    ApiAccountMeta:
      type: object
      description: Account metadata returned from instruction-building endpoints.
      required:
        - pubkey
        - isSigner
        - isWritable
      properties:
        isSigner:
          type: boolean
        isWritable:
          type: boolean
        pubkey:
          type: string
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/build-attached-conditional-order-transaction.md -->

---

# Source: `api/trader/build-cancel-conditional-order-transaction.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/build-cancel-conditional-order-transaction.md`

**Local path:** `api/trader/build-cancel-conditional-order-transaction.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Build cancel conditional order transaction

> Handles `POST /v1/ix/cancel-conditional-order` via `post.v1.ix.cancel_conditional_order`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json post /v1/ix/cancel-conditional-order
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/ix/cancel-conditional-order:
    post:
      tags:
        - Trader
      summary: Build cancel conditional order transaction
      description: >-
        Handles `POST /v1/ix/cancel-conditional-order` via
        `post.v1.ix.cancel_conditional_order`.
      operationId: post.v1.ix.cancel_conditional_order
      requestBody:
        description: JSON request payload for `post.v1.ix.cancel_conditional_order`.
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/CancelConditionalOrderRequest'
            example:
              authority: string
              conditionalOrderIndex: 0
              executionDirection: string
              symbol: string
              traderPdaIndex: 0
        required: true
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/ApiInstructionResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    CancelConditionalOrderRequest:
      type: object
      description: Request payload for /v1/ix/cancel-conditional-order.
      required:
        - authority
        - traderPdaIndex
        - symbol
        - conditionalOrderIndex
        - executionDirection
      properties:
        authority:
          type: string
        conditionalOrderIndex:
          type: integer
          format: int32
          minimum: 0
        executionDirection:
          type: string
        isIsolated:
          type: boolean
        positionAuthority:
          type:
            - string
            - 'null'
        symbol:
          type: string
        traderPdaIndex:
          type: integer
          format: int32
          minimum: 0
        traderSubaccountIndex:
          type:
            - integer
            - 'null'
          format: int32
          minimum: 0
    ApiInstructionResponse:
      type: object
      description: |-
        API representation of a Solana instruction.

        Note: this is a pure DTO. When needed, clients/servers can enable the
        appropriate conversion logic in the consumer crate to avoid forcing a
        Solana dependency in `phoenix-api-types`.
      required:
        - data
        - keys
        - programId
      properties:
        data:
          type: array
          items:
            type: integer
            format: int32
            minimum: 0
        keys:
          type: array
          items:
            $ref: '#/components/schemas/ApiAccountMeta'
        programId:
          type: string
    ApiAccountMeta:
      type: object
      description: Account metadata returned from instruction-building endpoints.
      required:
        - pubkey
        - isSigner
        - isWritable
      properties:
        isSigner:
          type: boolean
        isWritable:
          type: boolean
        pubkey:
          type: string
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/build-cancel-conditional-order-transaction.md -->

---

# Source: `api/trader/build-cancel-stop-loss-order-transaction.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/build-cancel-stop-loss-order-transaction.md`

**Local path:** `api/trader/build-cancel-stop-loss-order-transaction.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Build cancel stop loss order transaction

> Handles `POST /v1/ix/cancel-stop-loss-order` via `post.v1.ix.cancel_stop_loss_order`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json post /v1/ix/cancel-stop-loss-order
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/ix/cancel-stop-loss-order:
    post:
      tags:
        - Trader
      summary: Build cancel stop loss order transaction
      description: >-
        Handles `POST /v1/ix/cancel-stop-loss-order` via
        `post.v1.ix.cancel_stop_loss_order`.
      operationId: post.v1.ix.cancel_stop_loss_order
      requestBody:
        description: JSON request payload for `post.v1.ix.cancel_stop_loss_order`.
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/CancelStopLossOrderRequest'
            example:
              authority: string
              executionDirection: string
              symbol: string
              traderPdaIndex: 0
        required: true
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/ApiInstructionResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    CancelStopLossOrderRequest:
      type: object
      description: Request payload for /ix/cancel-stop-loss-order.
      required:
        - authority
        - traderPdaIndex
        - symbol
        - executionDirection
      properties:
        authority:
          type: string
        executionDirection:
          type: string
          description: >-
            Which trigger direction to cancel: e.g. "greater_than" or
            "less_than".
        isIsolated:
          type: boolean
        positionAuthority:
          type:
            - string
            - 'null'
        symbol:
          type: string
        traderPdaIndex:
          type: integer
          format: int32
          minimum: 0
        traderSubaccountIndex:
          type:
            - integer
            - 'null'
          format: int32
          minimum: 0
    ApiInstructionResponse:
      type: object
      description: |-
        API representation of a Solana instruction.

        Note: this is a pure DTO. When needed, clients/servers can enable the
        appropriate conversion logic in the consumer crate to avoid forcing a
        Solana dependency in `phoenix-api-types`.
      required:
        - data
        - keys
        - programId
      properties:
        data:
          type: array
          items:
            type: integer
            format: int32
            minimum: 0
        keys:
          type: array
          items:
            $ref: '#/components/schemas/ApiAccountMeta'
        programId:
          type: string
    ApiAccountMeta:
      type: object
      description: Account metadata returned from instruction-building endpoints.
      required:
        - pubkey
        - isSigner
        - isWritable
      properties:
        isSigner:
          type: boolean
        isWritable:
          type: boolean
        pubkey:
          type: string
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/build-cancel-stop-loss-order-transaction.md -->

---

# Source: `api/trader/build-isolated-limit-order-transaction-with-conditionals.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/build-isolated-limit-order-transaction-with-conditionals.md`

**Local path:** `api/trader/build-isolated-limit-order-transaction-with-conditionals.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Build isolated limit order transaction with conditionals

> Handles `POST /v1/ix/place-isolated-limit-order-with-conditionals` via `post.v1.ix.place_isolated_limit_order_with_conditionals`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json post /v1/ix/place-isolated-limit-order-with-conditionals
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/ix/place-isolated-limit-order-with-conditionals:
    post:
      tags:
        - Trader
      summary: Build isolated limit order transaction with conditionals
      description: >-
        Handles `POST /v1/ix/place-isolated-limit-order-with-conditionals` via
        `post.v1.ix.place_isolated_limit_order_with_conditionals`.
      operationId: post.v1.ix.place_isolated_limit_order_with_conditionals
      requestBody:
        description: >-
          JSON request payload for
          `post.v1.ix.place_isolated_limit_order_with_conditionals`.
        content:
          application/json:
            schema:
              $ref: >-
                #/components/schemas/PlaceIsolatedLimitOrderWithConditionalsRequest
            example:
              authority: string
              side: string
              symbol: string
        required: true
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/ApiInstructionResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    PlaceIsolatedLimitOrderWithConditionalsRequest:
      type: object
      description: Request payload for /v1/ix/place-isolated-limit-order-with-conditionals.
      required:
        - authority
        - symbol
        - side
      properties:
        allowCrossAndIsolatedForAsset:
          type:
            - boolean
            - 'null'
        authority:
          type: string
        feePayer:
          type:
            - string
            - 'null'
        flightBuilderAuthority:
          type:
            - string
            - 'null'
        flightFeeCollectorTrader:
          type:
            - string
            - 'null'
        greaterTrigger:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/ConditionalTriggerRequest'
        isPostOnly:
          type:
            - boolean
            - 'null'
          description: >-
            If true, the order will be post-only (maker only, will not match
            against

            existing orders).
        lessTrigger:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/ConditionalTriggerRequest'
        numBaseLots:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        pdaIndex:
          type:
            - integer
            - 'null'
          format: int32
          minimum: 0
        positionAuthority:
          type:
            - string
            - 'null'
        price:
          type:
            - number
            - 'null'
          format: double
        priceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        quantity:
          type:
            - number
            - 'null'
          format: double
        side:
          type: string
        skipTransferToParent:
          type:
            - boolean
            - 'null'
        slide:
          type:
            - boolean
            - 'null'
          description: |-
            For post-only orders: if true, slide to best price when order would
            cross. Defaults to true.
        symbol:
          type: string
        transferAmount:
          type: integer
          format: int64
          minimum: 0
    ApiInstructionResponse:
      type: object
      description: |-
        API representation of a Solana instruction.

        Note: this is a pure DTO. When needed, clients/servers can enable the
        appropriate conversion logic in the consumer crate to avoid forcing a
        Solana dependency in `phoenix-api-types`.
      required:
        - data
        - keys
        - programId
      properties:
        data:
          type: array
          items:
            type: integer
            format: int32
            minimum: 0
        keys:
          type: array
          items:
            $ref: '#/components/schemas/ApiAccountMeta'
        programId:
          type: string
    ConditionalTriggerRequest:
      type: object
      description: Trigger configuration for attached conditional orders.
      required:
        - side
      properties:
        executionPrice:
          type:
            - number
            - 'null'
          format: double
        executionPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        orderKind:
          type:
            - string
            - 'null'
        side:
          type: string
        triggerPrice:
          type:
            - number
            - 'null'
          format: double
        triggerPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
    ApiAccountMeta:
      type: object
      description: Account metadata returned from instruction-building endpoints.
      required:
        - pubkey
        - isSigner
        - isWritable
      properties:
        isSigner:
          type: boolean
        isWritable:
          type: boolean
        pubkey:
          type: string
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/build-isolated-limit-order-transaction-with-conditionals.md -->

---

# Source: `api/trader/build-isolated-limit-order-transaction-with-liquidation-estimate.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/build-isolated-limit-order-transaction-with-liquidation-estimate.md`

**Local path:** `api/trader/build-isolated-limit-order-transaction-with-liquidation-estimate.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Build isolated limit order transaction with liquidation estimate

> Handles `POST /v1/ix/place-isolated-limit-order-enhanced` via `post.v1.ix.place_isolated_limit_order_enhanced`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json post /v1/ix/place-isolated-limit-order-enhanced
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/ix/place-isolated-limit-order-enhanced:
    post:
      tags:
        - Trader
      summary: Build isolated limit order transaction with liquidation estimate
      description: >-
        Handles `POST /v1/ix/place-isolated-limit-order-enhanced` via
        `post.v1.ix.place_isolated_limit_order_enhanced`.
      operationId: post.v1.ix.place_isolated_limit_order_enhanced
      requestBody:
        description: >-
          JSON request payload for
          `post.v1.ix.place_isolated_limit_order_enhanced`.
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/PlaceIsolatedLimitOrderRequest'
            example:
              authority: string
              side: string
              symbol: string
        required: true
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/PlaceIsolatedLimitOrderEnhancedResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    PlaceIsolatedLimitOrderRequest:
      type: object
      description: Request payload for /ix/place-isolated-limit-order.
      required:
        - authority
        - symbol
        - side
      properties:
        allowCrossAndIsolatedForAsset:
          type:
            - boolean
            - 'null'
        authority:
          type: string
        feePayer:
          type:
            - string
            - 'null'
        flightBuilderAuthority:
          type:
            - string
            - 'null'
        flightFeeCollectorTrader:
          type:
            - string
            - 'null'
        isPostOnly:
          type:
            - boolean
            - 'null'
          description: >-
            If true, the order will be post-only (maker only, will not match
            against

            existing orders).
        isReduceOnly:
          type:
            - boolean
            - 'null'
        numBaseLots:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        pdaIndex:
          type:
            - integer
            - 'null'
          format: int32
          minimum: 0
        positionAuthority:
          type:
            - string
            - 'null'
        price:
          type:
            - number
            - 'null'
          format: double
        priceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        quantity:
          type:
            - number
            - 'null'
          format: double
        side:
          type: string
        skipTransferToParent:
          type:
            - boolean
            - 'null'
        slide:
          type:
            - boolean
            - 'null'
          description: |-
            For post-only orders: if true, slide to best price when order would
            cross. Defaults to true.
        symbol:
          type: string
        tpSl:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/TpSlOrderConfig'
        transferAmount:
          type: integer
          format: int64
          minimum: 0
    PlaceIsolatedLimitOrderEnhancedResponse:
      type: object
      required:
        - instructions
      properties:
        estimatedLiquidationPriceUsd:
          type:
            - number
            - 'null'
          format: double
        instructions:
          type: array
          items:
            $ref: '#/components/schemas/ApiInstructionResponse'
    TpSlOrderConfig:
      type: object
      description: TP/SL configuration shared across endpoints.
      properties:
        numBaseLots:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        orderKind:
          type:
            - string
            - 'null'
        quantity:
          type:
            - number
            - 'null'
          format: double
        stopLossExecutionPrice:
          type:
            - number
            - 'null'
          format: double
        stopLossExecutionPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        stopLossTriggerPrice:
          type:
            - number
            - 'null'
          format: double
        stopLossTriggerPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        takeProfitExecutionPrice:
          type:
            - number
            - 'null'
          format: double
        takeProfitExecutionPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        takeProfitTriggerPrice:
          type:
            - number
            - 'null'
          format: double
        takeProfitTriggerPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
    ApiInstructionResponse:
      type: object
      description: |-
        API representation of a Solana instruction.

        Note: this is a pure DTO. When needed, clients/servers can enable the
        appropriate conversion logic in the consumer crate to avoid forcing a
        Solana dependency in `phoenix-api-types`.
      required:
        - data
        - keys
        - programId
      properties:
        data:
          type: array
          items:
            type: integer
            format: int32
            minimum: 0
        keys:
          type: array
          items:
            $ref: '#/components/schemas/ApiAccountMeta'
        programId:
          type: string
    ApiAccountMeta:
      type: object
      description: Account metadata returned from instruction-building endpoints.
      required:
        - pubkey
        - isSigner
        - isWritable
      properties:
        isSigner:
          type: boolean
        isWritable:
          type: boolean
        pubkey:
          type: string
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/build-isolated-limit-order-transaction-with-liquidation-estimate.md -->

---

# Source: `api/trader/build-isolated-limit-order-transaction.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/build-isolated-limit-order-transaction.md`

**Local path:** `api/trader/build-isolated-limit-order-transaction.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Build isolated limit order transaction

> Handles `POST /v1/ix/place-isolated-limit-order` via `post.v1.ix.place_isolated_limit_order`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json post /v1/ix/place-isolated-limit-order
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/ix/place-isolated-limit-order:
    post:
      tags:
        - Trader
      summary: Build isolated limit order transaction
      description: >-
        Handles `POST /v1/ix/place-isolated-limit-order` via
        `post.v1.ix.place_isolated_limit_order`.
      operationId: post.v1.ix.place_isolated_limit_order
      requestBody:
        description: JSON request payload for `post.v1.ix.place_isolated_limit_order`.
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/PlaceIsolatedLimitOrderRequest'
            example:
              authority: string
              side: string
              symbol: string
        required: true
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/ApiInstructionResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    PlaceIsolatedLimitOrderRequest:
      type: object
      description: Request payload for /ix/place-isolated-limit-order.
      required:
        - authority
        - symbol
        - side
      properties:
        allowCrossAndIsolatedForAsset:
          type:
            - boolean
            - 'null'
        authority:
          type: string
        feePayer:
          type:
            - string
            - 'null'
        flightBuilderAuthority:
          type:
            - string
            - 'null'
        flightFeeCollectorTrader:
          type:
            - string
            - 'null'
        isPostOnly:
          type:
            - boolean
            - 'null'
          description: >-
            If true, the order will be post-only (maker only, will not match
            against

            existing orders).
        isReduceOnly:
          type:
            - boolean
            - 'null'
        numBaseLots:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        pdaIndex:
          type:
            - integer
            - 'null'
          format: int32
          minimum: 0
        positionAuthority:
          type:
            - string
            - 'null'
        price:
          type:
            - number
            - 'null'
          format: double
        priceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        quantity:
          type:
            - number
            - 'null'
          format: double
        side:
          type: string
        skipTransferToParent:
          type:
            - boolean
            - 'null'
        slide:
          type:
            - boolean
            - 'null'
          description: |-
            For post-only orders: if true, slide to best price when order would
            cross. Defaults to true.
        symbol:
          type: string
        tpSl:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/TpSlOrderConfig'
        transferAmount:
          type: integer
          format: int64
          minimum: 0
    ApiInstructionResponse:
      type: object
      description: |-
        API representation of a Solana instruction.

        Note: this is a pure DTO. When needed, clients/servers can enable the
        appropriate conversion logic in the consumer crate to avoid forcing a
        Solana dependency in `phoenix-api-types`.
      required:
        - data
        - keys
        - programId
      properties:
        data:
          type: array
          items:
            type: integer
            format: int32
            minimum: 0
        keys:
          type: array
          items:
            $ref: '#/components/schemas/ApiAccountMeta'
        programId:
          type: string
    TpSlOrderConfig:
      type: object
      description: TP/SL configuration shared across endpoints.
      properties:
        numBaseLots:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        orderKind:
          type:
            - string
            - 'null'
        quantity:
          type:
            - number
            - 'null'
          format: double
        stopLossExecutionPrice:
          type:
            - number
            - 'null'
          format: double
        stopLossExecutionPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        stopLossTriggerPrice:
          type:
            - number
            - 'null'
          format: double
        stopLossTriggerPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        takeProfitExecutionPrice:
          type:
            - number
            - 'null'
          format: double
        takeProfitExecutionPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        takeProfitTriggerPrice:
          type:
            - number
            - 'null'
          format: double
        takeProfitTriggerPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
    ApiAccountMeta:
      type: object
      description: Account metadata returned from instruction-building endpoints.
      required:
        - pubkey
        - isSigner
        - isWritable
      properties:
        isSigner:
          type: boolean
        isWritable:
          type: boolean
        pubkey:
          type: string
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/build-isolated-limit-order-transaction.md -->

---

# Source: `api/trader/build-isolated-market-order-transaction-with-liquidation-estimate.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/build-isolated-market-order-transaction-with-liquidation-estimate.md`

**Local path:** `api/trader/build-isolated-market-order-transaction-with-liquidation-estimate.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Build isolated market order transaction with liquidation estimate

> Handles `POST /v1/ix/place-isolated-market-order-enhanced` via `post.v1.ix.place_isolated_market_order_enhanced`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json post /v1/ix/place-isolated-market-order-enhanced
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/ix/place-isolated-market-order-enhanced:
    post:
      tags:
        - Trader
      summary: Build isolated market order transaction with liquidation estimate
      description: >-
        Handles `POST /v1/ix/place-isolated-market-order-enhanced` via
        `post.v1.ix.place_isolated_market_order_enhanced`.
      operationId: post.v1.ix.place_isolated_market_order_enhanced
      requestBody:
        description: >-
          JSON request payload for
          `post.v1.ix.place_isolated_market_order_enhanced`.
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/PlaceIsolatedMarketOrderRequest'
            example:
              authority: string
              side: string
              symbol: string
        required: true
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/PlaceIsolatedMarketOrderEnhancedResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    PlaceIsolatedMarketOrderRequest:
      type: object
      description: Request payload for /ix/place-isolated-market-order.
      required:
        - authority
        - symbol
        - side
      properties:
        allowCrossAndIsolatedForAsset:
          type:
            - boolean
            - 'null'
        authority:
          type: string
        feePayer:
          type:
            - string
            - 'null'
        flightBuilderAuthority:
          type:
            - string
            - 'null'
        flightFeeCollectorTrader:
          type:
            - string
            - 'null'
        isReduceOnly:
          type:
            - boolean
            - 'null'
        maxPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        numBaseLots:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        pdaIndex:
          type:
            - integer
            - 'null'
          format: int32
          minimum: 0
        positionAuthority:
          type:
            - string
            - 'null'
        quantity:
          type:
            - number
            - 'null'
          format: double
        side:
          type: string
        skipTransferToParent:
          type:
            - boolean
            - 'null'
        symbol:
          type: string
        tpSl:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/TpSlOrderConfig'
        transferAmount:
          type: integer
          format: int64
          minimum: 0
    PlaceIsolatedMarketOrderEnhancedResponse:
      type: object
      required:
        - instructions
      properties:
        estimatedLiquidationPriceUsd:
          type:
            - number
            - 'null'
          format: double
        instructions:
          type: array
          items:
            $ref: '#/components/schemas/ApiInstructionResponse'
    TpSlOrderConfig:
      type: object
      description: TP/SL configuration shared across endpoints.
      properties:
        numBaseLots:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        orderKind:
          type:
            - string
            - 'null'
        quantity:
          type:
            - number
            - 'null'
          format: double
        stopLossExecutionPrice:
          type:
            - number
            - 'null'
          format: double
        stopLossExecutionPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        stopLossTriggerPrice:
          type:
            - number
            - 'null'
          format: double
        stopLossTriggerPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        takeProfitExecutionPrice:
          type:
            - number
            - 'null'
          format: double
        takeProfitExecutionPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        takeProfitTriggerPrice:
          type:
            - number
            - 'null'
          format: double
        takeProfitTriggerPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
    ApiInstructionResponse:
      type: object
      description: |-
        API representation of a Solana instruction.

        Note: this is a pure DTO. When needed, clients/servers can enable the
        appropriate conversion logic in the consumer crate to avoid forcing a
        Solana dependency in `phoenix-api-types`.
      required:
        - data
        - keys
        - programId
      properties:
        data:
          type: array
          items:
            type: integer
            format: int32
            minimum: 0
        keys:
          type: array
          items:
            $ref: '#/components/schemas/ApiAccountMeta'
        programId:
          type: string
    ApiAccountMeta:
      type: object
      description: Account metadata returned from instruction-building endpoints.
      required:
        - pubkey
        - isSigner
        - isWritable
      properties:
        isSigner:
          type: boolean
        isWritable:
          type: boolean
        pubkey:
          type: string
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/build-isolated-market-order-transaction-with-liquidation-estimate.md -->

---

# Source: `api/trader/build-isolated-market-order-transaction.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/build-isolated-market-order-transaction.md`

**Local path:** `api/trader/build-isolated-market-order-transaction.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Build isolated market order transaction

> Handles `POST /v1/ix/place-isolated-market-order` via `post.v1.ix.place_isolated_market_order`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json post /v1/ix/place-isolated-market-order
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/ix/place-isolated-market-order:
    post:
      tags:
        - Trader
      summary: Build isolated market order transaction
      description: >-
        Handles `POST /v1/ix/place-isolated-market-order` via
        `post.v1.ix.place_isolated_market_order`.
      operationId: post.v1.ix.place_isolated_market_order
      requestBody:
        description: JSON request payload for `post.v1.ix.place_isolated_market_order`.
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/PlaceIsolatedMarketOrderRequest'
            example:
              authority: string
              side: string
              symbol: string
        required: true
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/ApiInstructionResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    PlaceIsolatedMarketOrderRequest:
      type: object
      description: Request payload for /ix/place-isolated-market-order.
      required:
        - authority
        - symbol
        - side
      properties:
        allowCrossAndIsolatedForAsset:
          type:
            - boolean
            - 'null'
        authority:
          type: string
        feePayer:
          type:
            - string
            - 'null'
        flightBuilderAuthority:
          type:
            - string
            - 'null'
        flightFeeCollectorTrader:
          type:
            - string
            - 'null'
        isReduceOnly:
          type:
            - boolean
            - 'null'
        maxPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        numBaseLots:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        pdaIndex:
          type:
            - integer
            - 'null'
          format: int32
          minimum: 0
        positionAuthority:
          type:
            - string
            - 'null'
        quantity:
          type:
            - number
            - 'null'
          format: double
        side:
          type: string
        skipTransferToParent:
          type:
            - boolean
            - 'null'
        symbol:
          type: string
        tpSl:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/TpSlOrderConfig'
        transferAmount:
          type: integer
          format: int64
          minimum: 0
    ApiInstructionResponse:
      type: object
      description: |-
        API representation of a Solana instruction.

        Note: this is a pure DTO. When needed, clients/servers can enable the
        appropriate conversion logic in the consumer crate to avoid forcing a
        Solana dependency in `phoenix-api-types`.
      required:
        - data
        - keys
        - programId
      properties:
        data:
          type: array
          items:
            type: integer
            format: int32
            minimum: 0
        keys:
          type: array
          items:
            $ref: '#/components/schemas/ApiAccountMeta'
        programId:
          type: string
    TpSlOrderConfig:
      type: object
      description: TP/SL configuration shared across endpoints.
      properties:
        numBaseLots:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        orderKind:
          type:
            - string
            - 'null'
        quantity:
          type:
            - number
            - 'null'
          format: double
        stopLossExecutionPrice:
          type:
            - number
            - 'null'
          format: double
        stopLossExecutionPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        stopLossTriggerPrice:
          type:
            - number
            - 'null'
          format: double
        stopLossTriggerPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        takeProfitExecutionPrice:
          type:
            - number
            - 'null'
          format: double
        takeProfitExecutionPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        takeProfitTriggerPrice:
          type:
            - number
            - 'null'
          format: double
        takeProfitTriggerPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
    ApiAccountMeta:
      type: object
      description: Account metadata returned from instruction-building endpoints.
      required:
        - pubkey
        - isSigner
        - isWritable
      properties:
        isSigner:
          type: boolean
        isWritable:
          type: boolean
        pubkey:
          type: string
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/build-isolated-market-order-transaction.md -->

---

# Source: `api/trader/build-position-conditional-order-transaction.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/build-position-conditional-order-transaction.md`

**Local path:** `api/trader/build-position-conditional-order-transaction.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Build position conditional order transaction

> Handles `POST /v1/ix/place-position-conditional-order` via `post.v1.ix.place_position_conditional_order`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json post /v1/ix/place-position-conditional-order
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/ix/place-position-conditional-order:
    post:
      tags:
        - Trader
      summary: Build position conditional order transaction
      description: >-
        Handles `POST /v1/ix/place-position-conditional-order` via
        `post.v1.ix.place_position_conditional_order`.
      operationId: post.v1.ix.place_position_conditional_order
      requestBody:
        description: >-
          JSON request payload for
          `post.v1.ix.place_position_conditional_order`.
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/PlacePositionConditionalOrderRequest'
            example:
              authority: string
              symbol: string
              traderPdaIndex: 0
        required: true
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/ApiInstructionResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    PlacePositionConditionalOrderRequest:
      type: object
      description: Request payload for /v1/ix/place-position-conditional-order.
      required:
        - authority
        - traderPdaIndex
        - symbol
      properties:
        authority:
          type: string
        feePayer:
          type:
            - string
            - 'null'
        flightBuilderAuthority:
          type:
            - string
            - 'null'
        flightFeeCollectorTrader:
          type:
            - string
            - 'null'
        greaterTrigger:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/ConditionalTriggerRequest'
        isIsolated:
          type: boolean
        lessTrigger:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/ConditionalTriggerRequest'
        numBaseLots:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        positionAuthority:
          type:
            - string
            - 'null'
        quantity:
          type:
            - number
            - 'null'
          format: double
        sizePercent:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        symbol:
          type: string
        traderPdaIndex:
          type: integer
          format: int32
          minimum: 0
        traderSubaccountIndex:
          type:
            - integer
            - 'null'
          format: int32
          minimum: 0
    ApiInstructionResponse:
      type: object
      description: |-
        API representation of a Solana instruction.

        Note: this is a pure DTO. When needed, clients/servers can enable the
        appropriate conversion logic in the consumer crate to avoid forcing a
        Solana dependency in `phoenix-api-types`.
      required:
        - data
        - keys
        - programId
      properties:
        data:
          type: array
          items:
            type: integer
            format: int32
            minimum: 0
        keys:
          type: array
          items:
            $ref: '#/components/schemas/ApiAccountMeta'
        programId:
          type: string
    ConditionalTriggerRequest:
      type: object
      description: Trigger configuration for attached conditional orders.
      required:
        - side
      properties:
        executionPrice:
          type:
            - number
            - 'null'
          format: double
        executionPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        orderKind:
          type:
            - string
            - 'null'
        side:
          type: string
        triggerPrice:
          type:
            - number
            - 'null'
          format: double
        triggerPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
    ApiAccountMeta:
      type: object
      description: Account metadata returned from instruction-building endpoints.
      required:
        - pubkey
        - isSigner
        - isWritable
      properties:
        isSigner:
          type: boolean
        isWritable:
          type: boolean
        pubkey:
          type: string
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/build-position-conditional-order-transaction.md -->

---

# Source: `api/trader/build-stop-loss-order-transaction.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/build-stop-loss-order-transaction.md`

**Local path:** `api/trader/build-stop-loss-order-transaction.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Build stop loss order transaction

> Handles `POST /v1/ix/place-stop-loss-order` via `post.v1.ix.place_stop_loss_order`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json post /v1/ix/place-stop-loss-order
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/ix/place-stop-loss-order:
    post:
      tags:
        - Trader
      summary: Build stop loss order transaction
      description: >-
        Handles `POST /v1/ix/place-stop-loss-order` via
        `post.v1.ix.place_stop_loss_order`.
      operationId: post.v1.ix.place_stop_loss_order
      requestBody:
        description: JSON request payload for `post.v1.ix.place_stop_loss_order`.
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/PlaceStopLossOrderRequest'
            example:
              numBaseLots: 0
        required: true
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/ApiInstructionResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    PlaceStopLossOrderRequest:
      allOf:
        - $ref: '#/components/schemas/TpSlOrderConfig'
        - type: object
          required:
            - authority
            - traderPdaIndex
            - symbol
            - side
          properties:
            authority:
              type: string
            feePayer:
              type:
                - string
                - 'null'
            flightBuilderAuthority:
              type:
                - string
                - 'null'
            flightFeeCollectorTrader:
              type:
                - string
                - 'null'
            isIsolated:
              type: boolean
            positionAuthority:
              type:
                - string
                - 'null'
            side:
              type: string
            symbol:
              type: string
            traderPdaIndex:
              type: integer
              format: int32
              minimum: 0
            traderSubaccountIndex:
              type:
                - integer
                - 'null'
              format: int32
              minimum: 0
      description: Request payload for /ix/place-stop-loss-order.
    ApiInstructionResponse:
      type: object
      description: |-
        API representation of a Solana instruction.

        Note: this is a pure DTO. When needed, clients/servers can enable the
        appropriate conversion logic in the consumer crate to avoid forcing a
        Solana dependency in `phoenix-api-types`.
      required:
        - data
        - keys
        - programId
      properties:
        data:
          type: array
          items:
            type: integer
            format: int32
            minimum: 0
        keys:
          type: array
          items:
            $ref: '#/components/schemas/ApiAccountMeta'
        programId:
          type: string
    TpSlOrderConfig:
      type: object
      description: TP/SL configuration shared across endpoints.
      properties:
        numBaseLots:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        orderKind:
          type:
            - string
            - 'null'
        quantity:
          type:
            - number
            - 'null'
          format: double
        stopLossExecutionPrice:
          type:
            - number
            - 'null'
          format: double
        stopLossExecutionPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        stopLossTriggerPrice:
          type:
            - number
            - 'null'
          format: double
        stopLossTriggerPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        takeProfitExecutionPrice:
          type:
            - number
            - 'null'
          format: double
        takeProfitExecutionPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
        takeProfitTriggerPrice:
          type:
            - number
            - 'null'
          format: double
        takeProfitTriggerPriceInTicks:
          type:
            - integer
            - 'null'
          format: int64
          minimum: 0
    ApiAccountMeta:
      type: object
      description: Account metadata returned from instruction-building endpoints.
      required:
        - pubkey
        - isSigner
        - isWritable
      properties:
        isSigner:
          type: boolean
        isWritable:
          type: boolean
        pubkey:
          type: string
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/build-stop-loss-order-transaction.md -->

---

# Source: `api/trader/get-order-history-v2.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/get-order-history-v2.md`

**Local path:** `api/trader/get-order-history-v2.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get order history v2

> Handles `GET /v1/traders/{trader_pubkey}/orders_v2` via `get.v1.traders.by_trader_pubkey.orders_v2`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/traders/{trader_pubkey}/orders_v2
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/traders/{trader_pubkey}/orders_v2:
    get:
      tags:
        - Trader
      summary: Get order history v2
      description: >-
        Handles `GET /v1/traders/{trader_pubkey}/orders_v2` via
        `get.v1.traders.by_trader_pubkey.orders_v2`.
      operationId: get.v1.traders.by_trader_pubkey.orders_v2
      parameters:
        - name: market_symbol
          in: query
          description: Optional market symbol filter
          required: false
          schema:
            type: string
        - name: limit
          in: query
          description: Number of items to return (max 1000)
          required: true
          schema:
            type: integer
            format: int64
        - name: cursor
          in: query
          description: >-
            Optional base64-encoded cursor for pagination. Returns items older
            than (exclusive of) this cursor.
          required: false
          schema:
            type: string
        - name: trader_pubkey
          in: path
          required: true
          schema:
            type: string
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/PaginatedResponse_Vec_OrderHistoryV2Item'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    PaginatedResponse_Vec_OrderHistoryV2Item:
      type: object
      description: >-
        Generic paginated response wrapper with bidirectional cursor support.


        The cursor system supports both forward (newer) and backward (older)

        pagination:

        - `prev_cursor`: Use this cursor to poll for new items (items newer than
        the
          current result set)
        - `next_cursor`: Use this cursor to load more items (items older than
        the
          current result set)

        The direction is embedded in the cursor itself, so clients just need to
        pass

        the appropriate cursor to the `cursor` parameter.
      required:
        - data
        - hasMore
      properties:
        data:
          type: array
          items:
            type: object
            description: >-
              Order history V2 item showing intended order (from packet), actual
              placed

              order (from event), and current state (from orders table)
            required:
              - userId
              - traderId
              - traderPdaIndex
              - traderSubaccountIndex
              - slot
              - slotIndex
              - instructionIndex
              - eventIndex
              - marketSymbol
              - instructionType
              - orderType
              - createdAt
              - price
              - intended
              - isReduceOnly
              - isStopLoss
              - isStopLossDirection
              - isConditionalOrder
              - status
            properties:
              createdAt:
                type: string
                format: date-time
              currentOrderState:
                oneOf:
                  - type: 'null'
                  - $ref: '#/components/schemas/CurrentOrderState'
              eventIndex:
                type: integer
                format: int32
              fillBeforePlacement:
                oneOf:
                  - type: 'null'
                  - $ref: '#/components/schemas/AggregatedTrades'
              instructionIndex:
                type: integer
                format: int32
              instructionType:
                type: string
              intended:
                $ref: '#/components/schemas/IntendedOrder'
              isConditionalOrder:
                type: boolean
              isReduceOnly:
                type: boolean
              isStopLoss:
                type: boolean
              isStopLossDirection:
                type: boolean
              marketSymbol:
                type: string
              orderType:
                type: string
              placedOrder:
                oneOf:
                  - type: 'null'
                  - $ref: '#/components/schemas/PlacedOrder'
              price:
                type: string
              slot:
                type: integer
                format: int64
              slotIndex:
                type: integer
                format: int32
              status:
                type: string
              traderId:
                type: integer
                format: int64
              traderPdaIndex:
                type: integer
                format: int32
              traderSubaccountIndex:
                type: integer
                format: int32
              userId:
                type: integer
                format: int64
        hasMore:
          type: boolean
          description: Whether there are more results available after this page
        nextCursor:
          type:
            - string
            - 'null'
          description: >-
            Opaque cursor for fetching the next page of older results.

            Pass this value as the `cursor` parameter in the next request to
            load

            more.
        prevCursor:
          type:
            - string
            - 'null'
          description: >-
            Opaque cursor for fetching newer items (for polling).

            Pass this value as the `cursor` parameter to get items newer than
            the

            first item in data.
    CurrentOrderState:
      type: object
      description: Current order state from orders table
      required:
        - orderSequenceNumber
        - side
        - price
        - baseQty
        - remainingBaseQty
        - filledBaseQty
        - releasedBaseQty
        - orderStatus
      properties:
        baseQty:
          type: string
        cancelledAt:
          type:
            - string
            - 'null'
          format: date-time
        filledBaseQty:
          type: string
        lastFillAt:
          type:
            - string
            - 'null'
          format: date-time
        orderSequenceNumber:
          type: integer
          format: int64
        orderStatus:
          type: string
          description: '"active", "filled", "cancelled"'
        placedAt:
          type:
            - string
            - 'null'
          format: date-time
        price:
          type: string
        releasedBaseQty:
          type: string
          description: base_qty - remaining_base_qty
        remainingBaseQty:
          type: string
        side:
          type: string
    AggregatedTrades:
      type: object
      description: Aggregated trades summary for an instruction
      required:
        - totalBaseQtyFilled
        - totalQuoteQtyFilled
        - totalTakerFee
        - totalNumFills
        - numTrades
      properties:
        numTrades:
          type: integer
          format: int32
        totalBaseQtyFilled:
          type: string
        totalNumFills:
          type: integer
          format: int32
        totalQuoteQtyFilled:
          type: string
        totalTakerFee:
          type: string
    IntendedOrder:
      type: object
      description: Intended order details from order_packets
      required:
        - orderKind
        - side
        - hasPriceLimit
        - baseLots
        - baseQty
        - orderFlags
        - isReduceOnly
        - isStopLoss
        - isPostOnly
        - isTakeOnly
        - cancelExisting
      properties:
        baseLots:
          type: integer
          format: int64
        baseQty:
          type: string
        cancelExisting:
          type: boolean
        clientOrderId:
          type:
            - string
            - 'null'
        hasPriceLimit:
          type: boolean
        isConditionalOrder:
          type: boolean
        isPostOnly:
          type: boolean
        isReduceOnly:
          type: boolean
        isStopLoss:
          type: boolean
        isStopLossDirection:
          type: boolean
        isTakeOnly:
          type: boolean
        lastValidSlot:
          type:
            - integer
            - 'null'
          format: int64
        matchLimit:
          type:
            - integer
            - 'null'
          format: int64
        orderFlags:
          type: integer
          format: int32
        orderKind:
          type: string
        priceInTicks:
          type:
            - integer
            - 'null'
          format: int64
        priceUsd:
          type:
            - string
            - 'null'
        quoteLotBudget:
          type:
            - integer
            - 'null'
          format: int64
        side:
          type: string
    PlacedOrder:
      type: object
      description: Actual placed order details from order_events
      required:
        - orderSequenceNumber
        - eventIndex
        - side
        - price
        - baseQty
        - transactionTimestamp
      properties:
        baseQty:
          type: string
        eventIndex:
          type: integer
          format: int32
        initialSlot:
          type:
            - integer
            - 'null'
          format: int64
        lastValidSlot:
          type:
            - integer
            - 'null'
          format: int64
        orderFlags:
          type:
            - integer
            - 'null'
          format: int32
        orderSequenceNumber:
          type: integer
          format: int64
        price:
          type: string
        side:
          type: string
        transactionTimestamp:
          type: string
          format: date-time
          description: Formatted datetime string (ISO 8601).
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/get-order-history-v2.md -->

---

# Source: `api/trader/get-order-history.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/get-order-history.md`

**Local path:** `api/trader/get-order-history.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get order history

> Handles `GET /v1/trader/{authority}/order-history` via `get.v1.trader.by_authority.order_history`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/trader/{authority}/order-history
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/trader/{authority}/order-history:
    get:
      tags:
        - Trader
      summary: Get order history
      description: >-
        Handles `GET /v1/trader/{authority}/order-history` via
        `get.v1.trader.by_authority.order_history`.
      operationId: get.v1.trader.by_authority.order_history
      parameters:
        - name: authority
          in: path
          description: Authority pubkey
          required: true
          schema:
            type: string
        - name: traderPdaIndex
          in: query
          required: false
          schema:
            type: integer
            format: int32
            minimum: 0
        - name: marketSymbol
          in: query
          required: false
          schema:
            type: string
        - name: limit
          in: query
          required: true
          schema:
            type: integer
            format: int64
        - name: cursor
          in: query
          required: false
          schema:
            type: string
        - name: privyId
          in: query
          required: false
          schema:
            type: string
        - name: orderStatus
          in: query
          required: false
          schema:
            type: string
      responses:
        '200':
          description: Order history
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/PaginatedResponse_Vec_OrderHistoryItem'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    PaginatedResponse_Vec_OrderHistoryItem:
      type: object
      description: >-
        Generic paginated response wrapper with bidirectional cursor support.


        The cursor system supports both forward (newer) and backward (older)

        pagination:

        - `prev_cursor`: Use this cursor to poll for new items (items newer than
        the
          current result set)
        - `next_cursor`: Use this cursor to load more items (items older than
        the
          current result set)

        The direction is embedded in the cursor itself, so clients just need to
        pass

        the appropriate cursor to the `cursor` parameter.
      required:
        - data
        - hasMore
      properties:
        data:
          type: array
          items:
            type: object
            description: Individual order in order history
            required:
              - orderSequenceNumber
              - marketSymbol
              - status
              - side
              - isReduceOnly
              - price
              - baseQty
              - remainingBaseQty
              - filledBaseQty
            properties:
              baseQty:
                type: string
                description: Base quantity (human-readable, decimal format)
              completedAt:
                type:
                  - string
                  - 'null'
                format: date-time
                description: Timestamp when the order was completed (ISO 8601)
              filledBaseQty:
                type: string
                description: Total filled base quantity (human-readable, decimal format)
              isConditionalOrder:
                type: boolean
                description: >-
                  Indicates whether the order originated from a conditional
                  trigger
              isReduceOnly:
                type: boolean
                description: >-
                  Indicates whether the order was marked reduce-only at
                  placement time
              isStopLoss:
                type: boolean
                description: >-
                  Indicates whether the order originated from a stop loss
                  trigger
              isStopLossDirection:
                type: boolean
                description: Indicates whether the TP/SL trigger used stop-loss direction
              marketSymbol:
                type: string
                description: Market symbol (e.g., "SOL-PERP")
              orderSequenceNumber:
                type: string
                description: Order sequence number
              placedAt:
                type:
                  - string
                  - 'null'
                format: date-time
                description: Timestamp when the order was placed (ISO 8601)
              price:
                type: string
                description: Order price (human-readable, decimal format)
              remainingBaseQty:
                type: string
                description: Remaining base quantity (human-readable, decimal format)
              side:
                $ref: '#/components/schemas/Side'
                description: Order side ("buy" or "sell")
              status:
                $ref: '#/components/schemas/OrderStatus'
                description: Order status
        hasMore:
          type: boolean
          description: Whether there are more results available after this page
        nextCursor:
          type:
            - string
            - 'null'
          description: >-
            Opaque cursor for fetching the next page of older results.

            Pass this value as the `cursor` parameter in the next request to
            load

            more.
        prevCursor:
          type:
            - string
            - 'null'
          description: >-
            Opaque cursor for fetching newer items (for polling).

            Pass this value as the `cursor` parameter to get items newer than
            the

            first item in data.
    Side:
      type: string
      enum:
        - bid
        - ask
    OrderStatus:
      type: string
      enum:
        - active
        - cancelled
        - filled
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/get-order-history.md -->

---

# Source: `api/trader/get-trade-history.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/get-trade-history.md`

**Local path:** `api/trader/get-trade-history.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get trade history

> Handles `GET /v1/trader/{authority}/trades-history` via `get.v1.trader.by_authority.trades_history`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/trader/{authority}/trades-history
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/trader/{authority}/trades-history:
    get:
      tags:
        - Trader
      summary: Get trade history
      description: >-
        Handles `GET /v1/trader/{authority}/trades-history` via
        `get.v1.trader.by_authority.trades_history`.
      operationId: get.v1.trader.by_authority.trades_history
      parameters:
        - name: authority
          in: path
          description: Authority pubkey
          required: true
          schema:
            type: string
        - name: pdaIndex
          in: query
          required: false
          schema:
            type: integer
            format: int32
            minimum: 0
        - name: marketSymbol
          in: query
          required: false
          schema:
            type: string
        - name: limit
          in: query
          required: false
          schema:
            type: integer
            format: int64
        - name: cursor
          in: query
          required: false
          schema:
            type: string
      responses:
        '200':
          description: Trade history
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/PaginatedResponse_Vec_TradeHistoryV2Item'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    PaginatedResponse_Vec_TradeHistoryV2Item:
      type: object
      description: >-
        Generic paginated response wrapper with bidirectional cursor support.


        The cursor system supports both forward (newer) and backward (older)

        pagination:

        - `prev_cursor`: Use this cursor to poll for new items (items newer than
        the
          current result set)
        - `next_cursor`: Use this cursor to load more items (items older than
        the
          current result set)

        The direction is embedded in the cursor itself, so clients just need to
        pass

        the appropriate cursor to the `cursor` parameter.
      required:
        - data
        - hasMore
      properties:
        data:
          type: array
          items:
            type: object
            description: A single flattened trade history item with PnL and position data
            required:
              - userId
              - traderId
              - traderPdaIndex
              - subaccountIndex
              - marketSymbol
              - timestamp
              - slot
              - slotIndex
              - eventIndex
              - instructionIndex
              - instructionType
              - baseLotsBefore
              - baseLotsAfter
              - baseLotsDelta
              - virtualQuoteLotsBefore
              - virtualQuoteLotsAfter
              - virtualQuoteLotsDelta
              - price
              - realizedPnl
              - fees
              - liquidity
              - tradeType
            properties:
              baseLotsAfter:
                type: string
                description: Base lots after the trade (human readable)
              baseLotsBefore:
                type: string
                description: Base lots before the trade (human readable)
              baseLotsDelta:
                type: string
                description: Base lots delta (human readable, signed)
              eventIndex:
                type: integer
                format: int32
              fees:
                type: string
              fillId:
                type:
                  - string
                  - 'null'
                description: Deterministic UUID v3 derived from the raw fill coordinates.
              instructionIndex:
                type: integer
                format: int32
              instructionType:
                type: string
                description: Instruction type (e.g., "PlaceLimitOrder", "PlaceMarketOrder")
              liquidity:
                $ref: '#/components/schemas/LiquidityRole'
              marketSymbol:
                type: string
                description: Market symbol
              orderSequenceNumber:
                type:
                  - integer
                  - 'null'
                format: int64
              price:
                type: string
              realizedPnl:
                type: string
                description: Realized PnL from this trade (human readable USD)
              signature:
                type:
                  - string
                  - 'null'
                description: Transaction signature
              slot:
                type: integer
                format: int64
                description: Slot coordinates for cursor
              slotIndex:
                type: integer
                format: int32
              splineSequenceNumber:
                type:
                  - integer
                  - 'null'
                format: int64
              subaccountIndex:
                type: integer
                format: int32
                description: Subaccount index
              timestamp:
                type: string
                format: date-time
                description: Formatted datetime string (ISO 8601).
              tradeType:
                $ref: '#/components/schemas/TradeType'
              traderId:
                type: integer
                format: int64
                description: Trader ID
              traderPdaIndex:
                type: integer
                format: int32
                description: Trader PDA index
              userId:
                type: integer
                format: int64
                description: User ID
              virtualQuoteLotsAfter:
                type: string
                description: Virtual quote lots after (human readable)
              virtualQuoteLotsBefore:
                type: string
                description: Virtual quote lots before (human readable)
              virtualQuoteLotsDelta:
                type: string
                description: Virtual quote lots delta (human readable, signed)
        hasMore:
          type: boolean
          description: Whether there are more results available after this page
        nextCursor:
          type:
            - string
            - 'null'
          description: >-
            Opaque cursor for fetching the next page of older results.

            Pass this value as the `cursor` parameter in the next request to
            load

            more.
        prevCursor:
          type:
            - string
            - 'null'
          description: >-
            Opaque cursor for fetching newer items (for polling).

            Pass this value as the `cursor` parameter to get items newer than
            the

            first item in data.
    LiquidityRole:
      type: string
      description: Liquidity role for the tracked account's fill.
      enum:
        - maker
        - taker
    TradeType:
      type: string
      enum:
        - limit
        - market
        - liquidation
        - adl
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/get-trade-history.md -->

---

# Source: `api/trader/get-trader-capabilities.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/get-trader-capabilities.md`

**Local path:** `api/trader/get-trader-capabilities.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get trader capabilities

> Handles `GET /v1/view/trader-capabilities` via `get.v1.view.trader_capabilities`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/view/trader-capabilities
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/view/trader-capabilities:
    get:
      tags:
        - Trader
      summary: Get trader capabilities
      description: >-
        Handles `GET /v1/view/trader-capabilities` via
        `get.v1.view.trader_capabilities`.
      operationId: get.v1.view.trader_capabilities
      responses:
        '200':
          description: Trader capability descriptors
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/TraderCapabilitiesMetadata'
        '500':
          $ref: '#/components/responses/ErrorResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    TraderCapabilitiesMetadata:
      type: object
      description: Metadata response that enumerates all trader capability descriptors.
      required:
        - capabilities
      properties:
        capabilities:
          type: array
          items:
            $ref: '#/components/schemas/TraderCapabilityDescriptor'
          description: Supported capability descriptors.
    TraderCapabilityDescriptor:
      type: object
      description: >-
        Describes a trader capability in a forward-compatible, human readable
        way.
      required:
        - key
        - displayName
        - description
      properties:
        deprecated:
          type: boolean
          description: Whether this capability key is deprecated.
        description:
          type: string
          description: Human-readable capability description.
        displayName:
          type: string
          description: Human-readable display name.
        key:
          type: string
          description: Stable capability key.
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/get-trader-capabilities.md -->

---

# Source: `api/trader/get-trader-collateral-history-1.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/get-trader-collateral-history-1.md`

**Local path:** `api/trader/get-trader-collateral-history-1.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get trader collateral history

> Handles `GET /v1/traders/{trader_pubkey}/collateral-history` via `get.v1.traders.by_trader_pubkey.collateral_history`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/traders/{trader_pubkey}/collateral-history
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/traders/{trader_pubkey}/collateral-history:
    get:
      tags:
        - Trader
      summary: Get trader collateral history
      description: >-
        Handles `GET /v1/traders/{trader_pubkey}/collateral-history` via
        `get.v1.traders.by_trader_pubkey.collateral_history`.
      operationId: get.v1.traders.by_trader_pubkey.collateral_history
      parameters:
        - name: limit
          in: query
          description: Number of items to return (max 1000)
          required: true
          schema:
            type: integer
            format: int64
        - name: nextCursor
          in: query
          description: Cursor for older events (base64-encoded).
          required: false
          schema:
            type: string
        - name: prevCursor
          in: query
          description: Cursor for newer events (base64-encoded).
          required: false
          schema:
            type: string
        - name: cursor
          in: query
          description: Deprecated cursor parameter (older events).
          required: false
          schema:
            type: string
        - name: trader_pubkey
          in: path
          required: true
          schema:
            type: string
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/CollateralHistoryResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    CollateralHistoryResponse:
      type: object
      description: Response for collateral event history queries
      required:
        - data
        - hasMore
      properties:
        data:
          type: array
          items:
            $ref: '#/components/schemas/CollateralEvent'
          description: The data payload (array of items)
        hasMore:
          type: boolean
          description: Whether there are more results in the requested direction
        nextCursor:
          type:
            - string
            - 'null'
          description: Cursor for fetching older results
        prevCursor:
          type:
            - string
            - 'null'
          description: Cursor for fetching newer results
    CollateralEvent:
      type: object
      description: A single collateral event (deposit or withdrawal)
      required:
        - slot
        - slotIndex
        - eventIndex
        - traderPdaIndex
        - traderSubaccountIndex
        - eventType
        - amount
        - collateralAfter
        - timestamp
      properties:
        amount:
          type: integer
          format: int64
          description: Amount deposited or withdrawn (in quote lots, 6 decimals)
        collateralAfter:
          type: integer
          format: int64
          description: Collateral balance after this event (in quote lots, 6 decimals)
        eventIndex:
          type: integer
          format: int32
          description: Event index for ordering within the slot
        eventType:
          type: string
          description: 'Event type: ''deposit'' or ''withdrawal'''
        slot:
          type: integer
          format: int64
          description: Solana slot when the event occurred
        slotIndex:
          type: integer
          format: int32
          description: Index within the slot
        timestamp:
          type: string
          format: date-time
          description: Formatted datetime string (ISO 8601).
        traderPdaIndex:
          type: integer
          format: int32
          description: Trader PDA index (usually 0)
        traderSubaccountIndex:
          type: integer
          format: int32
          description: Trader subaccount index
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/get-trader-collateral-history-1.md -->

---

# Source: `api/trader/get-trader-collateral-history.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/get-trader-collateral-history.md`

**Local path:** `api/trader/get-trader-collateral-history.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get trader collateral history

> Handles `GET /v1/trader/{authority}/collateral-history` via `get.v1.trader.by_authority.collateral_history`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/trader/{authority}/collateral-history
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/trader/{authority}/collateral-history:
    get:
      tags:
        - Trader
      summary: Get trader collateral history
      description: >-
        Handles `GET /v1/trader/{authority}/collateral-history` via
        `get.v1.trader.by_authority.collateral_history`.
      operationId: get.v1.trader.by_authority.collateral_history
      parameters:
        - name: authority
          in: path
          description: >-
            Trader authority pubkey. Use
            /v1/traders/{trader_pubkey}/collateral-history for direct trader PDA
            lookups.
          required: true
          schema:
            type: string
        - name: pdaIndex
          in: query
          required: false
          schema:
            type: integer
            format: int32
            minimum: 0
        - name: nextCursor
          in: query
          required: false
          schema:
            type: string
        - name: prevCursor
          in: query
          required: false
          schema:
            type: string
        - name: cursor
          in: query
          required: false
          schema:
            type: string
        - name: limit
          in: query
          required: true
          schema:
            type: integer
            format: int64
      responses:
        '200':
          description: Collateral history
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/CollateralHistoryResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    CollateralHistoryResponse:
      type: object
      description: Response for collateral event history queries
      required:
        - data
        - hasMore
      properties:
        data:
          type: array
          items:
            $ref: '#/components/schemas/CollateralEvent'
          description: The data payload (array of items)
        hasMore:
          type: boolean
          description: Whether there are more results in the requested direction
        nextCursor:
          type:
            - string
            - 'null'
          description: Cursor for fetching older results
        prevCursor:
          type:
            - string
            - 'null'
          description: Cursor for fetching newer results
    CollateralEvent:
      type: object
      description: A single collateral event (deposit or withdrawal)
      required:
        - slot
        - slotIndex
        - eventIndex
        - traderPdaIndex
        - traderSubaccountIndex
        - eventType
        - amount
        - collateralAfter
        - timestamp
      properties:
        amount:
          type: integer
          format: int64
          description: Amount deposited or withdrawn (in quote lots, 6 decimals)
        collateralAfter:
          type: integer
          format: int64
          description: Collateral balance after this event (in quote lots, 6 decimals)
        eventIndex:
          type: integer
          format: int32
          description: Event index for ordering within the slot
        eventType:
          type: string
          description: 'Event type: ''deposit'' or ''withdrawal'''
        slot:
          type: integer
          format: int64
          description: Solana slot when the event occurred
        slotIndex:
          type: integer
          format: int32
          description: Index within the slot
        timestamp:
          type: string
          format: date-time
          description: Formatted datetime string (ISO 8601).
        traderPdaIndex:
          type: integer
          format: int32
          description: Trader PDA index (usually 0)
        traderSubaccountIndex:
          type: integer
          format: int32
          description: Trader subaccount index
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/get-trader-collateral-history.md -->

---

# Source: `api/trader/get-trader-funding-history.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/get-trader-funding-history.md`

**Local path:** `api/trader/get-trader-funding-history.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get trader funding history

> Handles `GET /v1/trader/{authority}/funding-history` via `get.v1.trader.by_authority.funding_history`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/trader/{authority}/funding-history
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/trader/{authority}/funding-history:
    get:
      tags:
        - Trader
      summary: Get trader funding history
      description: >-
        Handles `GET /v1/trader/{authority}/funding-history` via
        `get.v1.trader.by_authority.funding_history`.
      operationId: get.v1.trader.by_authority.funding_history
      parameters:
        - name: authority
          in: path
          description: Authority pubkey
          required: true
          schema:
            type: string
        - name: traderPdaIndex
          in: query
          required: false
          schema:
            type: integer
            format: int32
        - name: symbol
          in: query
          required: false
          schema:
            type: string
        - name: startTime
          in: query
          required: false
          schema:
            type: integer
            format: int64
        - name: endTime
          in: query
          required: false
          schema:
            type: integer
            format: int64
        - name: limit
          in: query
          required: false
          schema:
            type: integer
            format: int64
        - name: cursor
          in: query
          required: false
          schema:
            type: string
        - name: resolution
          in: query
          required: false
          schema:
            type: string
      responses:
        '200':
          description: Funding history
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/TraderFundingHistoryResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    TraderFundingHistoryResponse:
      type: object
      required:
        - events
        - hasMore
      properties:
        events:
          type: array
          items:
            $ref: '#/components/schemas/TraderFundingHistoryEvent'
        hasMore:
          type: boolean
        nextCursor:
          type:
            - string
            - 'null'
        prevCursor:
          type:
            - string
            - 'null'
    TraderFundingHistoryEvent:
      type: object
      required:
        - timestamp
        - symbol
        - fundingPayment
        - fundingRatePercentage
        - positionSize
        - positionSide
      properties:
        fundingPayment:
          type: string
        fundingRatePercentage:
          type: string
        positionSide:
          type: string
        positionSize:
          type: string
        symbol:
          type: string
        timestamp:
          type: string
          format: date-time
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/get-trader-funding-history.md -->

---

# Source: `api/trader/get-trader-market-pnl.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/get-trader-market-pnl.md`

**Local path:** `api/trader/get-trader-market-pnl.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get trader market PnL

> Handles `GET /v1/traders/{trader_pubkey}/pnl/markets` via `get.v1.traders.by_trader_pubkey.pnl.markets`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/traders/{trader_pubkey}/pnl/markets
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/traders/{trader_pubkey}/pnl/markets:
    get:
      tags:
        - Trader
      summary: Get trader market PnL
      description: >-
        Handles `GET /v1/traders/{trader_pubkey}/pnl/markets` via
        `get.v1.traders.by_trader_pubkey.pnl.markets`.
      operationId: get.v1.traders.by_trader_pubkey.pnl.markets
      parameters:
        - name: trader_pubkey
          in: path
          description: Trader public key (base58 encoded)
          required: true
          schema:
            type: string
        - name: resolution
          in: query
          description: 'Resolution/timeframe: 1m, 5m, 15m, 1h, 4h, 1d'
          required: true
          schema:
            type: string
        - name: startTime
          in: query
          description: 'Start time in milliseconds since Unix epoch (max range: 1 year)'
          required: false
          schema:
            type: integer
            format: int64
        - name: endTime
          in: query
          description: 'End time in milliseconds since Unix epoch (max range: 1 year)'
          required: false
          schema:
            type: integer
            format: int64
        - name: limit
          in: query
          description: 'Max number of data points per market (default: 1000, max: 1000)'
          required: false
          schema:
            type: integer
            format: int64
        - name: symbols
          in: query
          description: Comma separated list of market symbols to filter (defaults to all)
          required: false
          schema:
            type: string
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/TraderMarketPnLSeries'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    TraderMarketPnLSeries:
      type: object
      description: Per-market PnL time-series for a trader
      required:
        - marketId
        - symbol
        - tickSize
        - baseDecimals
        - points
      properties:
        baseDecimals:
          type: integer
          format: int32
          description: Base decimals used for this market
        marketId:
          type: integer
          format: int64
          description: Market identifier
        points:
          type: array
          items:
            $ref: '#/components/schemas/TraderMarketPnLPoint'
          description: Time-series points
        symbol:
          type: string
          description: Market symbol (e.g. BTC, SOL, ETH)
        tickSize:
          type: integer
          format: int32
          description: Tick size in quote lots per base lot per tick
    TraderMarketPnLPoint:
      type: object
      description: Per-market PnL point including position context
      required:
        - timestamp
        - startTime
        - endTime
        - realizedPnl
        - cumulativePnl
        - cumulativeFundingPayment
        - totalTakerFee
        - cumulativeTakerFee
        - unrealizedPnl
        - baseLots
        - virtualQuoteLots
        - markPrice
      properties:
        baseLots:
          type: integer
          format: int64
          description: Base lots position at the end of the bucket
        cumulativeFundingPayment:
          type: number
          format: double
          description: Cumulative funding payments up to this point
        cumulativePnl:
          type: number
          format: double
          description: Cumulative PnL up to this point
        cumulativeTakerFee:
          type: number
          format: double
          description: Cumulative taker fees paid up to this point
        endTime:
          type: integer
          format: int64
          description: End time in seconds since Unix epoch
        markPrice:
          type: integer
          format: int64
          description: Mark price in ticks at the end of the bucket
        realizedPnl:
          type: number
          format: double
          description: Realized PnL accrued inside the bucket
        startTime:
          type: integer
          format: int64
          description: Start time in seconds since Unix epoch
        timestamp:
          type: integer
          format: int64
          description: 'Deprecated: Unix timestamp in seconds.'
        totalTakerFee:
          type: number
          format: double
          description: Taker fees paid inside the bucket
        unrealizedPnl:
          type: number
          format: double
          description: Unrealized PnL at the end of the bucket
        virtualQuoteLots:
          type: integer
          format: int64
          description: Virtual quote lots position at the end of the bucket
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/get-trader-market-pnl.md -->

---

# Source: `api/trader/get-trader-pnl.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/get-trader-pnl.md`

**Local path:** `api/trader/get-trader-pnl.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get trader PnL

> Handles `GET /v1/traders/{trader_pubkey}/pnl` via `get.v1.traders.by_trader_pubkey.pnl`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/traders/{trader_pubkey}/pnl
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/traders/{trader_pubkey}/pnl:
    get:
      tags:
        - Trader
      summary: Get trader PnL
      description: >-
        Handles `GET /v1/traders/{trader_pubkey}/pnl` via
        `get.v1.traders.by_trader_pubkey.pnl`.
      operationId: get.v1.traders.by_trader_pubkey.pnl
      parameters:
        - name: trader_pubkey
          in: path
          description: Trader public key (base58 encoded)
          required: true
          schema:
            type: string
        - name: resolution
          in: query
          description: 'Resolution/timeframe: 1m, 1d'
          required: true
          schema:
            type: string
        - name: startTime
          in: query
          description: 'Start time in milliseconds since Unix epoch (max range: 1 year)'
          required: false
          schema:
            type: integer
            format: int64
        - name: endTime
          in: query
          description: 'End time in milliseconds since Unix epoch (max range: 1 year)'
          required: false
          schema:
            type: integer
            format: int64
        - name: limit
          in: query
          description: 'Max number of data points (default: 1000, max: 1440)'
          required: false
          schema:
            type: integer
            format: int64
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/PnLPoint'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    PnLPoint:
      type: object
      description: PnL data point
      required:
        - timestamp
        - startTime
        - endTime
        - cumulativePnl
        - unrealizedPnl
        - cumulativeFundingPayment
        - cumulativeTakerFee
      properties:
        cumulativeFundingPayment:
          type: number
          format: double
          description: Cumulative funding payment up to this point
        cumulativePnl:
          type: number
          format: double
          description: Cumulative realized PnL up to this point
        cumulativeTakerFee:
          type: number
          format: double
          description: Cumulative taker fees paid up to this point
        endTime:
          type: integer
          format: int64
          description: End time in seconds since Unix epoch
        startTime:
          type: integer
          format: int64
          description: Start time in seconds since Unix epoch
        timestamp:
          type: integer
          format: int64
          description: 'Deprecated: Unix timestamp in seconds.'
        unrealizedPnl:
          type: number
          format: double
          description: unrealized PnL up to this point
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/get-trader-pnl.md -->

---

# Source: `api/trader/get-trader-portfolio-values.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/get-trader-portfolio-values.md`

**Local path:** `api/trader/get-trader-portfolio-values.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get trader portfolio values

> Handles `GET /v1/traders/{trader_pubkey}/portfolio-values` via `get.v1.traders.by_trader_pubkey.portfolio_values`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/traders/{trader_pubkey}/portfolio-values
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/traders/{trader_pubkey}/portfolio-values:
    get:
      tags:
        - Trader
      summary: Get trader portfolio values
      description: >-
        Handles `GET /v1/traders/{trader_pubkey}/portfolio-values` via
        `get.v1.traders.by_trader_pubkey.portfolio_values`.
      operationId: get.v1.traders.by_trader_pubkey.portfolio_values
      parameters:
        - name: trader_pubkey
          in: path
          description: Trader public key (base58 encoded)
          required: true
          schema:
            type: string
        - name: resolution
          in: query
          description: >-
            Resolution/timeframe: 1m (testing only), 15m, 30m, 1h, 4h, 1d, 1w,
            1M (month)
          required: true
          schema:
            type: string
        - name: startTime
          in: query
          description: 'Start time in milliseconds since Unix epoch (max range: 1 year)'
          required: false
          schema:
            type: integer
            format: int64
        - name: endTime
          in: query
          description: 'End time in milliseconds since Unix epoch (max range: 1 year)'
          required: false
          schema:
            type: integer
            format: int64
        - name: limit
          in: query
          description: 'Max number of data points (default: 1440, max: 2000)'
          required: false
          schema:
            type: integer
            format: int64
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/PortfolioValuePoint'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    PortfolioValuePoint:
      type: object
      description: Portfolio value data point
      required:
        - timestamp
        - startTime
        - endTime
        - value
      properties:
        endTime:
          type: integer
          format: int64
          description: End time in seconds since Unix epoch
        positions:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/HashMap'
              description: Positions snapshot for this point in time
        startTime:
          type: integer
          format: int64
          description: Start time in seconds since Unix epoch
        timestamp:
          type: integer
          format: int64
          description: 'Deprecated: Unix timestamp in seconds.'
        value:
          type: number
          format: double
          description: Portfolio value as a floating-point number
    HashMap:
      type: object
      additionalProperties:
        type: object
        description: Position data for a single market stored in snapshots
        required:
          - baseLots
          - quoteLots
          - positionValue
          - initialMargin
        properties:
          baseLots:
            type: string
            description: Base lots as a formatted string
          initialMargin:
            type: string
            description: Initial margin as a formatted string
          positionValue:
            type: string
            description: Position value as a formatted string
          quoteLots:
            type: string
            description: Virtual quote lots as a formatted string
      propertyNames:
        type: string
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/get-trader-portfolio-values.md -->

---

# Source: `api/trader/get-trader-state.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/get-trader-state.md`

**Local path:** `api/trader/get-trader-state.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get trader state

> Handles `GET /v1/trader/state/{authority_pubkey}` via `get.v1.trader.state.by_authority_pubkey`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/trader/state/{authority_pubkey}
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/trader/state/{authority_pubkey}:
    get:
      tags:
        - Trader
      summary: Get trader state
      description: >-
        Handles `GET /v1/trader/state/{authority_pubkey}` via
        `get.v1.trader.state.by_authority_pubkey`.
      operationId: get.v1.trader.state.by_authority_pubkey
      parameters:
        - name: traderPdaIndex
          in: query
          description: Optional trader PDA index under the authority.
          required: false
          schema:
            type: integer
            format: int32
            minimum: 0
        - name: authority_pubkey
          in: path
          required: true
          schema:
            type: string
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/TraderStateSnapshotResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    TraderStateSnapshotResponse:
      type: object
      description: REST response wrapper for a trader-state snapshot.
      required:
        - authority
        - traderPdaIndex
        - slot
        - slotIndex
        - snapshot
      properties:
        authority:
          type: string
          description: Trader authority public key.
        slot:
          type: integer
          format: int64
          description: Solana slot for this snapshot.
          minimum: 0
        slotIndex:
          type: integer
          format: int32
          description: Slot-local index used for deterministic ordering.
          minimum: 0
        snapshot:
          $ref: '#/components/schemas/TraderStateSnapshot'
          description: Full trader-state snapshot.
        traderPdaIndex:
          type: integer
          format: int32
          description: Trader PDA index under the authority.
          minimum: 0
    TraderStateSnapshot:
      type: object
      description: Snapshot payload covering every subaccount belonging to a trader PDA.
      required:
        - version
        - capabilities
        - makerFeeOverrideMultiplier
        - takerFeeOverrideMultiplier
        - subaccounts
      properties:
        capabilities:
          $ref: '#/components/schemas/TraderStateCapabilities'
          description: >-
            Trader capability flags and derived views (shared across
            subaccounts).
        makerFeeOverrideMultiplier:
          type: number
          format: double
          description: >-
            Maker fee multiplier (1.0 = default, <1.0 = discount, >1.0 =
            premium).
        subaccounts:
          type: array
          items:
            $ref: '#/components/schemas/TraderStateSubaccountSnapshot'
          description: Per-subaccount snapshots for this trader PDA.
        takerFeeOverrideMultiplier:
          type: number
          format: double
          description: >-
            Taker fee multiplier (1.0 = default, <1.0 = discount, >1.0 =
            premium).
        version:
          type: integer
          format: int32
          description: Snapshot schema version.
          minimum: 0
    TraderStateCapabilities:
      type: object
      description: |-
        Trader capability flags for a subaccount along with convenient derived
        views.
      required:
        - flags
        - state
        - capabilities
      properties:
        capabilities:
          $ref: '#/components/schemas/TraderCapabilitiesView'
          description: Derived capability booleans.
        flags:
          $ref: '#/components/schemas/TraderCapabilityFlags'
          description: Raw trader capability bitflags.
        state:
          $ref: '#/components/schemas/TraderActivityStateView'
          description: Derived trader activity state.
    TraderStateSubaccountSnapshot:
      type: object
      description: >-
        Complete subaccount view contained in a snapshot.

        Snapshot of a single subaccount in the `traderState` channel.


        Captures collateral plus the latest known rows for every resource while
        also

        surfacing the per-subaccount sequence watermark so clients can detect
        drift.
      required:
        - subaccountIndex
        - sequence
        - collateral
      properties:
        capabilities:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/TraderStateCapabilities'
              description: >-
                Trader capability flags and derived views. Optional for
                backwards

                compatibility.
        collateral:
          type: string
          description: Collateral balance (human-readable decimal string).
        cooldownStatus:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/CooldownStatus'
              description: >-
                Withdrawal cooldown status. Optional for backwards
                compatibility.
        orders:
          type: array
          items:
            $ref: '#/components/schemas/TraderStateLimitOrderEvent'
          description: Limit-order rows grouped by symbol.
        positions:
          type: array
          items:
            $ref: '#/components/schemas/TraderStatePositionSnapshot'
          description: Position rows keyed by symbol.
        sequence:
          type: integer
          format: int64
          description: Monotonic sequence for subaccount state updates.
          minimum: 0
        splines:
          type: array
          items:
            $ref: '#/components/schemas/TraderStateSplineSnapshot'
          description: Spline rows grouped by symbol.
        subaccountIndex:
          type: integer
          format: int32
          description: Subaccount index under the trader PDA.
          minimum: 0
        triggers:
          type: array
          items:
            $ref: '#/components/schemas/TraderStateTriggerSnapshot'
          description: Trigger rows grouped by symbol (TP/SL and conditional triggers).
    TraderCapabilitiesView:
      type: object
      description: |-
        Capability matrix exposed alongside a trader's status. Each field is a
        nested object that reports whether the action is immediately available
        (`immediate`) and whether it becomes available after activating a cold
        trader (`viaColdActivation`).
      required:
        - placeLimitOrder
        - placeMarketOrder
        - riskIncreasingTrade
        - riskReducingTrade
        - depositCollateral
        - withdrawCollateral
      properties:
        depositCollateral:
          $ref: '#/components/schemas/CapabilityAccessView'
          description: Capability to deposit collateral.
        placeLimitOrder:
          $ref: '#/components/schemas/CapabilityAccessView'
          description: Capability to place limit orders.
        placeMarketOrder:
          $ref: '#/components/schemas/CapabilityAccessView'
          description: Capability to place market orders.
        riskIncreasingTrade:
          $ref: '#/components/schemas/CapabilityAccessView'
          description: Capability to execute risk-increasing trades.
        riskReducingTrade:
          $ref: '#/components/schemas/CapabilityAccessView'
          description: Capability to execute risk-reducing trades.
        withdrawCollateral:
          $ref: '#/components/schemas/CapabilityAccessView'
          description: Capability to withdraw collateral.
    TraderCapabilityFlags:
      type: integer
      format: int32
      description: >-
        Canonical capability mask stored in every trader header. The derived
        preset

        constructors (`cold`, `hot_active`, `reduce_only`, `frozen`) preserve
        the

        intended behavioural guarantees: cold traders operate from their local

        buffer until warmed, active traders have full matching-engine access,

        reduce-only traders may only take risk-reducing actions (but can still
        move

        collateral), and frozen traders are fully quarantined with balance
        transfers

        disabled.
      minimum: 0
    TraderActivityStateView:
      type: string
      description: Serializable representation of a trader's high-level activity state.
      enum:
        - uninitialized
        - cold
        - active
        - reduceOnly
        - frozen
    CooldownStatus:
      type: object
      description: >-
        Withdrawal cooldown status for a trader PDA.


        Indicates whether the trader can withdraw based on the time elapsed
        since

        their last deposit. This applies at the PDA level (main account), not

        per-subaccount.


        Clients should compute withdrawal eligibility as:

        `canWithdraw = (currentSlot >= lastDepositSlot + cooldownPeriodInSlots)
        &&

        capabilities.withdraw`
      required:
        - lastDepositSlot
        - cooldownPeriodInSlots
      properties:
        cooldownPeriodInSlots:
          type: integer
          format: int64
          description: Number of slots required to wait after a deposit before withdrawing.
          minimum: 0
        lastDepositSlot:
          type: integer
          format: int64
          description: Slot when the trader last deposited collateral.
          minimum: 0
    TraderStateLimitOrderEvent:
      type: object
      description: Order grouping used for snapshots.
      required:
        - symbol
        - orders
      properties:
        orders:
          type: array
          items:
            $ref: '#/components/schemas/TraderStateMarketLimitOrderEvent'
          description: Orders currently associated with the symbol.
        symbol:
          type: string
          description: Market symbol for this order group.
    TraderStatePositionSnapshot:
      allOf:
        - $ref: '#/components/schemas/TraderStatePositionRow'
          description: Position row values for the symbol.
        - type: object
          required:
            - symbol
          properties:
            symbol:
              type: string
              description: Market symbol for this position row.
      description: >-
        Snapshot entry keyed by market symbol.

        Flattened position entry keyed by market symbol.


        Used by snapshots so clients can store each symbol's latest values
        without

        dealing with nested arrays.
    TraderStateSplineSnapshot:
      allOf:
        - $ref: '#/components/schemas/TraderStateSplineRow'
          description: Spline row values for the symbol.
        - type: object
          required:
            - symbol
          properties:
            symbol:
              type: string
              description: Market symbol for this spline row.
      description: Spline snapshot grouped by market symbol.
    TraderStateTriggerSnapshot:
      allOf:
        - $ref: '#/components/schemas/TraderStateTriggerRow'
          description: Trigger rows for the symbol.
        - type: object
          required:
            - symbol
          properties:
            symbol:
              type: string
              description: Market symbol for this trigger group.
      description: >-
        Trigger snapshot grouped by market symbol.


        Top-level subaccount entry containing all TP/SL and conditional triggers

        for a single market. This is the canonical location for trigger data;

        the trigger fields still present on `TraderStatePositionRow` are
        deprecated.
    CapabilityAccessView:
      type: object
      description: >-
        Capability exposure for the current trader state. `immediate` indicates
        the

        action can be taken without additional state transitions, while

        `via_cold_activation` signals the action will succeed if the trader
        warms

        from `InitializedCold` to `InitializedHot` during processing.
      required:
        - immediate
      properties:
        immediate:
          type: boolean
          description: Whether this capability is currently available.
        viaColdActivation:
          type: boolean
          description: Whether this capability becomes available after cold activation.
    TraderStateMarketLimitOrderEvent:
      type: object
      description: |-
        Detailed limit order representation.
        Wire representation of a single limit-order row grouped by symbol.
      required:
        - orderSequenceNumber
        - side
        - orderType
        - priceTicks
        - priceUsd
        - sizeRemainingLots
        - initialSizeLots
        - reduceOnly
        - status
      properties:
        change:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/TraderStateRowChangeKind'
              description: Row-level change kind for delta payloads.
        conditionalKind:
          type:
            - string
            - 'null'
          description: Conditional order kind when present.
        initialSizeLots:
          type: string
          description: Initial order size in lots.
        isConditionalOrder:
          type: boolean
          description: Whether this order originated from a conditional trigger.
        isStopLoss:
          type: boolean
          description: Whether this order is a stop-loss order.
        isStopLossDirection:
          type: boolean
          description: Whether this TP/SL order uses stop-loss direction.
        orderSequenceNumber:
          type: string
          description: Order sequence number.
        orderType:
          type: string
          description: Order type (for example, limit/market variants).
        priceTicks:
          type: string
          description: Price in ticks.
        priceUsd:
          type: string
          description: Price in USD (decimal string).
        reduceOnly:
          type: boolean
          description: Whether the order is reduce-only.
        side:
          $ref: '#/components/schemas/Side'
          description: Side of the order.
        sizeRemainingLots:
          type: string
          description: Remaining order size in lots.
        status:
          type: string
          description: Current order status.
    TraderStatePositionRow:
      type: object
      description: Position row used for snapshots and deltas.
      required:
        - positionSequenceNumber
        - basePositionLots
        - entryPriceTicks
        - entryPriceUsd
        - virtualQuotePositionLots
        - unsettledFundingQuoteLots
        - accumulatedFundingQuoteLots
        - takeProfitTriggers
        - stopLossTriggers
      properties:
        accumulatedFundingQuoteLots:
          type: string
          description: Accumulated funding amount in quote lots (signed decimal string).
        basePositionLots:
          type: string
          description: Base position size in lots (signed decimal string).
        conditionalStopLossTriggers:
          type: array
          items:
            $ref: '#/components/schemas/TraderStateConditionalStopLossTrigger'
          description: Configured conditional stop-loss triggers.
        conditionalTakeProfitTriggers:
          type: array
          items:
            $ref: '#/components/schemas/TraderStateConditionalTakeProfitTrigger'
          description: Configured conditional take-profit triggers.
        entryPriceTicks:
          type: string
          description: Entry price in ticks.
        entryPriceUsd:
          type: string
          description: Entry price in USD (decimal string).
        positionSequenceNumber:
          type: string
          description: Position sequence number used for ordering/versioning.
        stopLossTriggers:
          type: array
          items:
            $ref: '#/components/schemas/TraderStateStopLossTrigger'
          description: Configured stop-loss triggers.
        takeProfitTriggers:
          type: array
          items:
            $ref: '#/components/schemas/TraderStateTakeProfitTrigger'
          description: Configured take-profit triggers.
        unsettledFundingQuoteLots:
          type: string
          description: Unsettled funding amount in quote lots (signed decimal string).
        virtualQuotePositionLots:
          type: string
          description: Virtual quote position in lots (signed decimal string).
    TraderStateSplineRow:
      type: object
      description: Spline row containing the spline parameters and fill state.
      required:
        - midPriceTicks
        - bidFilledAmountLots
        - askFilledAmountLots
      properties:
        askFilledAmountLots:
          type: string
          description: Total filled ask amount in lots.
        askRegions:
          type: array
          items:
            $ref: '#/components/schemas/TraderStateTickRegion'
          description: Ask-side spline regions.
        bidFilledAmountLots:
          type: string
          description: Total filled bid amount in lots.
        bidRegions:
          type: array
          items:
            $ref: '#/components/schemas/TraderStateTickRegion'
          description: Bid-side spline regions.
        midPriceTicks:
          type: string
          description: Mid price in ticks.
    TraderStateTriggerRow:
      type: object
      description: All trigger data for a single market symbol.
      required:
        - takeProfitTriggers
        - stopLossTriggers
      properties:
        conditionalStopLossTriggers:
          type: array
          items:
            $ref: '#/components/schemas/TraderStateConditionalStopLossTrigger'
          description: Configured conditional stop-loss triggers.
        conditionalTakeProfitTriggers:
          type: array
          items:
            $ref: '#/components/schemas/TraderStateConditionalTakeProfitTrigger'
          description: Configured conditional take-profit triggers.
        stopLossTriggers:
          type: array
          items:
            $ref: '#/components/schemas/TraderStateStopLossTrigger'
          description: Configured stop-loss triggers.
        takeProfitTriggers:
          type: array
          items:
            $ref: '#/components/schemas/TraderStateTakeProfitTrigger'
          description: Configured take-profit triggers.
    TraderStateRowChangeKind:
      type: string
      description: Change indicator used for row-level deltas.
      enum:
        - updated
        - closed
    Side:
      type: string
      enum:
        - bid
        - ask
    TraderStateConditionalStopLossTrigger:
      type: object
      description: Conditional stop-loss trigger rows scoped to a position.
      required:
        - conditionalStopLossId
        - trigger
        - status
      properties:
        conditionalStopLossId:
          type: string
          description: Conditional stop-loss trigger identifier.
        status:
          type: string
          description: Trigger status.
        trigger:
          $ref: '#/components/schemas/TraderStateConditionalTrigger'
          description: Trigger configuration and conditional metadata.
    TraderStateConditionalTakeProfitTrigger:
      type: object
      description: Conditional take-profit trigger rows scoped to a position.
      required:
        - conditionalTakeProfitId
        - trigger
        - status
      properties:
        conditionalTakeProfitId:
          type: string
          description: Conditional take-profit trigger identifier.
        status:
          type: string
          description: Trigger status.
        trigger:
          $ref: '#/components/schemas/TraderStateConditionalTrigger'
          description: Trigger configuration and conditional metadata.
    TraderStateStopLossTrigger:
      type: object
      description: Stop-loss trigger rows scoped to a position.
      required:
        - stopLossId
        - trigger
        - status
      properties:
        status:
          type: string
          description: Trigger status.
        stopLossId:
          type: string
          description: Stop-loss trigger identifier.
        trigger:
          $ref: '#/components/schemas/TraderStateTrigger'
          description: Trigger configuration.
    TraderStateTakeProfitTrigger:
      type: object
      description: Take-profit trigger rows scoped to a position.
      required:
        - takeProfitId
        - trigger
        - status
      properties:
        status:
          type: string
          description: Trigger status.
        takeProfitId:
          type: string
          description: Take-profit trigger identifier.
        trigger:
          $ref: '#/components/schemas/TraderStateTrigger'
          description: Trigger configuration.
    TraderStateTickRegion:
      type: object
      description: Tick region for spline configuration.
      required:
        - startPriceTicks
        - endPriceTicks
        - densityLotsPerTick
        - totalSizeLots
        - filledSizeLots
      properties:
        densityLotsPerTick:
          type: string
          description: Density in lots per tick.
        endPriceTicks:
          type: string
          description: End price of the region in ticks.
        filledSizeLots:
          type: string
          description: Filled region size in lots.
        startPriceTicks:
          type: string
          description: Start price of the region in ticks.
        totalSizeLots:
          type: string
          description: Total region size in lots.
    TraderStateConditionalTrigger:
      allOf:
        - $ref: '#/components/schemas/TraderStateTrigger'
          description: Base trigger configuration.
        - type: object
          required:
            - maxSizeLots
            - fillableSizeLots
            - filledSizeLots
            - usePercent
            - percent
          properties:
            attachedOrderSequenceNumber:
              type:
                - string
                - 'null'
              description: Attached order sequence number when present.
            fillableSizeLots:
              type: string
              description: Remaining fillable trigger size in base lots.
            filledSizeLots:
              type: string
              description: Filled trigger size in base lots.
            maxSizeLots:
              type: string
              description: Max trigger size in base lots.
            percent:
              type: integer
              format: int32
              description: Percentage used when `use_percent` is true.
              minimum: 0
            usePercent:
              type: boolean
              description: Whether trigger sizing uses a percentage of margin.
      description: Trigger configuration for conditional trigger orders.
    TraderStateTrigger:
      type: object
      description: Trigger configuration for TP/SL orders.
      required:
        - triggerPriceTicks
        - executionPriceTicks
        - side
        - kind
      properties:
        executionPriceTicks:
          type: string
          description: Execution price in ticks when the trigger fires.
        kind:
          $ref: '#/components/schemas/StopLossOrderKind'
          description: Stop-loss order kind.
        side:
          $ref: '#/components/schemas/Side'
          description: Order side for the trigger.
        triggerPriceTicks:
          type: string
          description: Trigger price in ticks.
    StopLossOrderKind:
      type: string
      enum:
        - ioc
        - limit
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/get-trader-state.md -->

---

# Source: `api/trader/get-trader-trade-history-v2.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/get-trader-trade-history-v2.md`

**Local path:** `api/trader/get-trader-trade-history-v2.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get trader trade history v2

> Handles `GET /v1/traders/{trader_pubkey}/trades_v2` via `get.v1.traders.by_trader_pubkey.trades_v2`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/traders/{trader_pubkey}/trades_v2
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/traders/{trader_pubkey}/trades_v2:
    get:
      tags:
        - Trader
      summary: Get trader trade history v2
      description: >-
        Handles `GET /v1/traders/{trader_pubkey}/trades_v2` via
        `get.v1.traders.by_trader_pubkey.trades_v2`.
      operationId: get.v1.traders.by_trader_pubkey.trades_v2
      parameters:
        - name: market_symbol
          in: query
          description: Optional market symbol to filter by (e.g., "SOL-PERP")
          required: false
          schema:
            type: string
        - name: trader_pda_index
          in: query
          description: Optional trader PDA index to scope results for a specific trader PDA
          required: false
          schema:
            type: integer
            format: int32
        - name: limit
          in: query
          description: Number of items to return (max 1000)
          required: false
          schema:
            type: integer
            format: int64
        - name: cursor
          in: query
          description: |-
            Optional cursor for pagination (format: "slot" or "slot,slot_index")
            Items returned will be older than (exclusive of) this cursor
          required: false
          schema:
            type: string
        - name: privy_id
          in: query
          description: Privy user ID (optional)
          required: false
          schema:
            type: string
        - name: trader_pubkey
          in: path
          required: true
          schema:
            type: string
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/PaginatedResponse_Vec_TradeHistoryV2Item'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    PaginatedResponse_Vec_TradeHistoryV2Item:
      type: object
      description: >-
        Generic paginated response wrapper with bidirectional cursor support.


        The cursor system supports both forward (newer) and backward (older)

        pagination:

        - `prev_cursor`: Use this cursor to poll for new items (items newer than
        the
          current result set)
        - `next_cursor`: Use this cursor to load more items (items older than
        the
          current result set)

        The direction is embedded in the cursor itself, so clients just need to
        pass

        the appropriate cursor to the `cursor` parameter.
      required:
        - data
        - hasMore
      properties:
        data:
          type: array
          items:
            type: object
            description: A single flattened trade history item with PnL and position data
            required:
              - userId
              - traderId
              - traderPdaIndex
              - subaccountIndex
              - marketSymbol
              - timestamp
              - slot
              - slotIndex
              - eventIndex
              - instructionIndex
              - instructionType
              - baseLotsBefore
              - baseLotsAfter
              - baseLotsDelta
              - virtualQuoteLotsBefore
              - virtualQuoteLotsAfter
              - virtualQuoteLotsDelta
              - price
              - realizedPnl
              - fees
              - liquidity
              - tradeType
            properties:
              baseLotsAfter:
                type: string
                description: Base lots after the trade (human readable)
              baseLotsBefore:
                type: string
                description: Base lots before the trade (human readable)
              baseLotsDelta:
                type: string
                description: Base lots delta (human readable, signed)
              eventIndex:
                type: integer
                format: int32
              fees:
                type: string
              fillId:
                type:
                  - string
                  - 'null'
                description: Deterministic UUID v3 derived from the raw fill coordinates.
              instructionIndex:
                type: integer
                format: int32
              instructionType:
                type: string
                description: Instruction type (e.g., "PlaceLimitOrder", "PlaceMarketOrder")
              liquidity:
                $ref: '#/components/schemas/LiquidityRole'
              marketSymbol:
                type: string
                description: Market symbol
              orderSequenceNumber:
                type:
                  - integer
                  - 'null'
                format: int64
              price:
                type: string
              realizedPnl:
                type: string
                description: Realized PnL from this trade (human readable USD)
              signature:
                type:
                  - string
                  - 'null'
                description: Transaction signature
              slot:
                type: integer
                format: int64
                description: Slot coordinates for cursor
              slotIndex:
                type: integer
                format: int32
              splineSequenceNumber:
                type:
                  - integer
                  - 'null'
                format: int64
              subaccountIndex:
                type: integer
                format: int32
                description: Subaccount index
              timestamp:
                type: string
                format: date-time
                description: Formatted datetime string (ISO 8601).
              tradeType:
                $ref: '#/components/schemas/TradeType'
              traderId:
                type: integer
                format: int64
                description: Trader ID
              traderPdaIndex:
                type: integer
                format: int32
                description: Trader PDA index
              userId:
                type: integer
                format: int64
                description: User ID
              virtualQuoteLotsAfter:
                type: string
                description: Virtual quote lots after (human readable)
              virtualQuoteLotsBefore:
                type: string
                description: Virtual quote lots before (human readable)
              virtualQuoteLotsDelta:
                type: string
                description: Virtual quote lots delta (human readable, signed)
        hasMore:
          type: boolean
          description: Whether there are more results available after this page
        nextCursor:
          type:
            - string
            - 'null'
          description: >-
            Opaque cursor for fetching the next page of older results.

            Pass this value as the `cursor` parameter in the next request to
            load

            more.
        prevCursor:
          type:
            - string
            - 'null'
          description: >-
            Opaque cursor for fetching newer items (for polling).

            Pass this value as the `cursor` parameter to get items newer than
            the

            first item in data.
    LiquidityRole:
      type: string
      description: Liquidity role for the tracked account's fill.
      enum:
        - maker
        - taker
    TradeType:
      type: string
      enum:
        - limit
        - market
        - liquidation
        - adl
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/get-trader-trade-history-v2.md -->

---

# Source: `api/trader/get-trader.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/get-trader.md`

**Local path:** `api/trader/get-trader.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get trader

> Handles `GET /v1/view/trader/{pubkey}` via `get.v1.view.trader.by_pubkey`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/view/trader/{pubkey}
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/view/trader/{pubkey}:
    get:
      tags:
        - Trader
      summary: Get trader
      description: >-
        Handles `GET /v1/view/trader/{pubkey}` via
        `get.v1.view.trader.by_pubkey`.
      operationId: get.v1.view.trader.by_pubkey
      parameters:
        - name: pubkey
          in: path
          description: Base58 encoded trader account pubkey
          required: true
          schema:
            type: string
      responses:
        '200':
          description: Phoenix Eternal trader
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/TraderView'
        '400':
          $ref: '#/components/responses/ErrorResponse'
        '500':
          $ref: '#/components/responses/ErrorResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    TraderView:
      type: object
      description: Trader view with all trading information
      required:
        - flags
        - state
        - capabilities
        - slot
        - slotIndex
        - traderKey
        - traderPdaIndex
        - traderSubaccountIndex
        - authority
        - collateralBalance
        - effectiveCollateral
        - effectiveCollateralForWithdrawals
        - unrealizedPnl
        - discountedUnrealizedPnl
        - unsettledFundingOwed
        - accumulatedFunding
        - portfolioValue
        - maintenanceMargin
        - cancelMargin
        - initialMargin
        - initialMarginForWithdrawals
        - riskState
        - riskTier
        - positions
        - limitOrders
        - makerFeeOverrideMultiplier
        - takerFeeOverrideMultiplier
        - maxPositions
        - lastDepositSlot
        - isInActiveTraders
        - numMarketsWithSplines
      properties:
        accumulatedFunding:
          type: string
          description: Accumulated funding amount.
        authority:
          type: string
          description: Trader authority public key.
        cancelMargin:
          type: string
          description: Margin threshold used for cancellation checks.
        capabilities:
          $ref: '#/components/schemas/TraderCapabilitiesView'
          description: Derived capability matrix.
        collateralBalance:
          type: string
          description: Collateral balance.
        discountedUnrealizedPnl:
          type: string
          description: Discounted unrealized PnL used in risk calculations.
        effectiveCollateral:
          type: string
          description: Effective collateral used for risk checks.
        effectiveCollateralForWithdrawals:
          type: string
          description: Effective collateral used for withdrawal checks.
        flags:
          $ref: '#/components/schemas/TraderCapabilityFlags'
          description: Raw trader capability flags.
        initialMargin:
          type: string
          description: Required initial margin.
        initialMarginForWithdrawals:
          type: string
          description: Initial margin used for withdrawal checks.
        isInActiveTraders:
          type: boolean
          description: Whether trader is currently in the active-trader buffer.
        lastDepositSlot:
          type: integer
          format: int64
          description: Last collateral deposit slot.
          minimum: 0
        limitOrders:
          type: object
          description: Open limit orders grouped by symbol.
          additionalProperties:
            type: array
            items:
              $ref: '#/components/schemas/LimitOrder'
          propertyNames:
            type: string
        maintenanceMargin:
          type: string
          description: Required maintenance margin.
        makerFeeOverrideMultiplier:
          type: number
          format: double
          description: >-
            Maker fee multiplier (1.0 = default, <1.0 = discount, >1.0 =
            premium).
        maxPositions:
          type: integer
          format: int64
          description: Maximum number of positions allowed.
          minimum: 0
        numMarketsWithSplines:
          type: integer
          format: int32
          description: Number of markets where this trader has registered splines
          minimum: 0
        portfolioValue:
          type: string
          description: Current portfolio value.
        positions:
          type: array
          items:
            $ref: '#/components/schemas/TraderPositionView'
          description: Open positions.
        riskState:
          $ref: '#/components/schemas/RiskState'
          description: Current risk state.
        riskTier:
          $ref: '#/components/schemas/RiskTier'
          description: Current risk tier.
        slot:
          type: integer
          format: int64
          description: Solana slot of the state snapshot used to build this trader view.
          minimum: 0
        slotIndex:
          type: integer
          format: int32
          description: |-
            Intra-slot sequence index of the state snapshot used to build this
            trader view.
          minimum: 0
        state:
          $ref: '#/components/schemas/TraderActivityStateView'
          description: Derived high-level trader state.
        takerFeeOverrideMultiplier:
          type: number
          format: double
          description: >-
            Taker fee multiplier (1.0 = default, <1.0 = discount, >1.0 =
            premium).
        traderKey:
          type: string
          description: Trader PDA public key.
        traderPdaIndex:
          type: integer
          format: int32
          description: Trader PDA index under the authority.
          minimum: 0
        traderSubaccountIndex:
          type: integer
          format: int32
          description: Trader subaccount index.
          minimum: 0
        unrealizedPnl:
          type: string
          description: Total unrealized PnL.
        unsettledFundingOwed:
          type: string
          description: >-
            Unsettled funding amount owed to the trader.

            - Positive value = funding you will receive when settled (increases
              collateral)
            - Negative value = funding you owe when settled (decreases
            collateral)
    TraderCapabilitiesView:
      type: object
      description: |-
        Capability matrix exposed alongside a trader's status. Each field is a
        nested object that reports whether the action is immediately available
        (`immediate`) and whether it becomes available after activating a cold
        trader (`viaColdActivation`).
      required:
        - placeLimitOrder
        - placeMarketOrder
        - riskIncreasingTrade
        - riskReducingTrade
        - depositCollateral
        - withdrawCollateral
      properties:
        depositCollateral:
          $ref: '#/components/schemas/CapabilityAccessView'
          description: Capability to deposit collateral.
        placeLimitOrder:
          $ref: '#/components/schemas/CapabilityAccessView'
          description: Capability to place limit orders.
        placeMarketOrder:
          $ref: '#/components/schemas/CapabilityAccessView'
          description: Capability to place market orders.
        riskIncreasingTrade:
          $ref: '#/components/schemas/CapabilityAccessView'
          description: Capability to execute risk-increasing trades.
        riskReducingTrade:
          $ref: '#/components/schemas/CapabilityAccessView'
          description: Capability to execute risk-reducing trades.
        withdrawCollateral:
          $ref: '#/components/schemas/CapabilityAccessView'
          description: Capability to withdraw collateral.
    TraderCapabilityFlags:
      type: integer
      format: int32
      description: >-
        Canonical capability mask stored in every trader header. The derived
        preset

        constructors (`cold`, `hot_active`, `reduce_only`, `frozen`) preserve
        the

        intended behavioural guarantees: cold traders operate from their local

        buffer until warmed, active traders have full matching-engine access,

        reduce-only traders may only take risk-reducing actions (but can still
        move

        collateral), and frozen traders are fully quarantined with balance
        transfers

        disabled.
      minimum: 0
    LimitOrder:
      type: object
      description: A trader's limit order.
      required:
        - price
        - side
        - orderSequenceNumber
        - initialTradeSize
        - tradeSizeRemaining
        - marginRequirement
        - marginFactor
        - isReduceOnly
      properties:
        initialTradeSize:
          type: string
        isConditionalOrder:
          type: boolean
        isReduceOnly:
          type: boolean
        isStopLoss:
          type: boolean
        isStopLossDirection:
          type: boolean
        marginFactor:
          type: string
          description: >-
            The margin factor applied to limit orders (0.75 = 75% margin
            required).
        marginRequirement:
          type: string
          description: The margin requirement for this specific limit order.
        orderSequenceNumber:
          type: string
        price:
          type: string
        side:
          $ref: '#/components/schemas/Side'
        tradeSizeRemaining:
          type: string
    TraderPositionView:
      type: object
      description: A trader's position view.
      required:
        - symbol
        - positionSize
        - virtualQuotePosition
        - entryPrice
        - unrealizedPnl
        - discountedUnrealizedPnl
        - positionInitialMargin
        - initialMargin
        - maintenanceMargin
        - backstopMargin
        - limitOrderMargin
        - positionValue
        - unsettledFunding
        - accumulatedFunding
        - liquidationPrice
      properties:
        accumulatedFunding:
          type: string
          description: Accumulated funding amount.
        backstopMargin:
          type: string
          description: The backstop margin threshold for transfer mechanism.
        discountedUnrealizedPnl:
          type: string
          description: Unrealized PnL after risk discounting.
        entryPrice:
          type: string
          description: Average entry price.
        initialMargin:
          type: string
          description: The total initial margin required for position + limit orders.
        limitOrderMargin:
          type: string
          description: The margin required for current limit orders.
        liquidationPrice:
          type: string
          description: Estimated liquidation price.
        maintenanceMargin:
          type: string
          description: The maintenance margin required to avoid liquidation.
        positionInitialMargin:
          type: string
          description: |-
            The initial margin required for just the position (excluding limit
            orders).
        positionSize:
          type: string
          description: Net position size in base units.
        positionValue:
          type: string
          description: Current position notional value.
        stopLossPrice:
          type:
            - string
            - 'null'
          description: Best stop-loss trigger price, if configured.
        symbol:
          type: string
          description: Market symbol (for example, "SOL-PERP").
        takeProfitPrice:
          type:
            - string
            - 'null'
          description: Best take-profit trigger price, if configured.
        unrealizedPnl:
          type: string
          description: Current unrealized PnL.
        unsettledFunding:
          type: string
          description: >-
            Unsettled funding amount.

            - Positive value = funding you will receive when settled (increases
              collateral)
            - Negative value = funding you owe when settled (decreases
            collateral)
        virtualQuotePosition:
          type: string
          description: Virtual quote position in quote units.
    RiskState:
      type: string
      enum:
        - healthy
        - unhealthy
        - underwater
        - zeroCollateralNoPositions
    RiskTier:
      type: string
      enum:
        - safe
        - atRisk
        - cancellable
        - liquidatable
        - backstopLiquidatable
        - highRisk
    TraderActivityStateView:
      type: string
      description: Serializable representation of a trader's high-level activity state.
      enum:
        - uninitialized
        - cold
        - active
        - reduceOnly
        - frozen
    ErrorResponse:
      type: object
      required:
        - error
      properties:
        error:
          type: string
    CapabilityAccessView:
      type: object
      description: >-
        Capability exposure for the current trader state. `immediate` indicates
        the

        action can be taken without additional state transitions, while

        `via_cold_activation` signals the action will succeed if the trader
        warms

        from `InitializedCold` to `InitializedHot` during processing.
      required:
        - immediate
      properties:
        immediate:
          type: boolean
          description: Whether this capability is currently available.
        viaColdActivation:
          type: boolean
          description: Whether this capability becomes available after cold activation.
    Side:
      type: string
      enum:
        - bid
        - ask
  responses:
    ErrorResponse:
      description: Standard JSON error payload.
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/get-trader.md -->

---

# Source: `api/trader/get-user-collateral-history.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/get-user-collateral-history.md`

**Local path:** `api/trader/get-user-collateral-history.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get user collateral history

> Handles `GET /v1/users/{user_pubkey}/collateral-history` via `get.v1.users.by_user_pubkey.collateral_history`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/users/{user_pubkey}/collateral-history
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/users/{user_pubkey}/collateral-history:
    get:
      tags:
        - Trader
      summary: Get user collateral history
      description: >-
        Handles `GET /v1/users/{user_pubkey}/collateral-history` via
        `get.v1.users.by_user_pubkey.collateral_history`.
      operationId: get.v1.users.by_user_pubkey.collateral_history
      parameters:
        - name: limit
          in: query
          description: Number of items to return (max 1000)
          required: true
          schema:
            type: integer
            format: int64
        - name: nextCursor
          in: query
          description: Cursor for older events (base64-encoded).
          required: false
          schema:
            type: string
        - name: prevCursor
          in: query
          description: Cursor for newer events (base64-encoded).
          required: false
          schema:
            type: string
        - name: cursor
          in: query
          description: Deprecated cursor parameter (older events).
          required: false
          schema:
            type: string
        - name: user_pubkey
          in: path
          required: true
          schema:
            type: string
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/CollateralHistoryResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    CollateralHistoryResponse:
      type: object
      description: Response for collateral event history queries
      required:
        - data
        - hasMore
      properties:
        data:
          type: array
          items:
            $ref: '#/components/schemas/CollateralEvent'
          description: The data payload (array of items)
        hasMore:
          type: boolean
          description: Whether there are more results in the requested direction
        nextCursor:
          type:
            - string
            - 'null'
          description: Cursor for fetching older results
        prevCursor:
          type:
            - string
            - 'null'
          description: Cursor for fetching newer results
    CollateralEvent:
      type: object
      description: A single collateral event (deposit or withdrawal)
      required:
        - slot
        - slotIndex
        - eventIndex
        - traderPdaIndex
        - traderSubaccountIndex
        - eventType
        - amount
        - collateralAfter
        - timestamp
      properties:
        amount:
          type: integer
          format: int64
          description: Amount deposited or withdrawn (in quote lots, 6 decimals)
        collateralAfter:
          type: integer
          format: int64
          description: Collateral balance after this event (in quote lots, 6 decimals)
        eventIndex:
          type: integer
          format: int32
          description: Event index for ordering within the slot
        eventType:
          type: string
          description: 'Event type: ''deposit'' or ''withdrawal'''
        slot:
          type: integer
          format: int64
          description: Solana slot when the event occurred
        slotIndex:
          type: integer
          format: int32
          description: Index within the slot
        timestamp:
          type: string
          format: date-time
          description: Formatted datetime string (ISO 8601).
        traderPdaIndex:
          type: integer
          format: int32
          description: Trader PDA index (usually 0)
        traderSubaccountIndex:
          type: integer
          format: int32
          description: Trader subaccount index
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/get-user-collateral-history.md -->

---

# Source: `api/trader/get-user-hourly-funding.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/get-user-hourly-funding.md`

**Local path:** `api/trader/get-user-hourly-funding.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get user hourly funding

> Handles `GET /v1/users/{user_pubkey}/funding-hourly` via `get.v1.users.by_user_pubkey.funding_hourly`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/users/{user_pubkey}/funding-hourly
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/users/{user_pubkey}/funding-hourly:
    get:
      tags:
        - Trader
      summary: Get user hourly funding
      description: >-
        Handles `GET /v1/users/{user_pubkey}/funding-hourly` via
        `get.v1.users.by_user_pubkey.funding_hourly`.
      operationId: get.v1.users.by_user_pubkey.funding_hourly
      parameters:
        - name: user_pubkey
          in: path
          description: User public key (base58 encoded)
          required: true
          schema:
            type: string
        - name: traderPdaIndex
          in: query
          description: Trader PDA index (default 0)
          required: false
          schema:
            type: integer
            format: int32
        - name: symbol
          in: query
          description: Optional market symbol (e.g., 'SOL-PERP')
          required: false
          schema:
            type: string
        - name: limit
          in: query
          description: 'Number of hourly rows to return (1-200, default: 50)'
          required: false
          schema:
            type: integer
            format: int64
        - name: cursor
          in: query
          description: Opaque pagination cursor (base64url-encoded; do not parse)
          required: false
          schema:
            type: string
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/FundingHourlyHistoryResponse'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    FundingHourlyHistoryResponse:
      type: object
      description: Response containing hourly funding accruals
      required:
        - events
        - hasMore
      properties:
        events:
          type: array
          items:
            $ref: '#/components/schemas/FundingHourlyEvent'
        hasMore:
          type: boolean
        nextCursor:
          type:
            - string
            - 'null'
          description: Cursor for fetching older items (for pagination)
        prevCursor:
          type:
            - string
            - 'null'
          description: Cursor for fetching newer items (for polling)
    FundingHourlyEvent:
      type: object
      description: Individual hourly funding accrual entry
      required:
        - timestamp
        - symbol
        - fundingPayment
        - fundingRatePercentage
        - positionSize
        - positionSide
      properties:
        fundingPayment:
          type: string
          description: >-
            Funding payment amount in USDC (negative = paid, positive =
            received)
        fundingRatePercentage:
          type: string
          description: Funding rate applied for the hour (percentage of USD notional)
        positionSide:
          type: string
          description: Position side ("Long", "Short", or "Flat")
        positionSize:
          type: string
          description: Position size in base units
        symbol:
          type: string
          description: Market symbol (e.g., "SOL-PERP")
        timestamp:
          type: string
          format: date-time
          description: Formatted datetime string (ISO 8601).
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/get-user-hourly-funding.md -->

---

# Source: `api/trader/get-user-liquidation-history.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/get-user-liquidation-history.md`

**Local path:** `api/trader/get-user-liquidation-history.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get user liquidation history

> Handles `GET /v1/users/{user_pubkey}/liquidation-history` via `get.v1.users.by_user_pubkey.liquidation_history`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/users/{user_pubkey}/liquidation-history
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/users/{user_pubkey}/liquidation-history:
    get:
      tags:
        - Trader
      summary: Get user liquidation history
      description: >-
        Handles `GET /v1/users/{user_pubkey}/liquidation-history` via
        `get.v1.users.by_user_pubkey.liquidation_history`.
      operationId: get.v1.users.by_user_pubkey.liquidation_history
      parameters:
        - name: pdaIndex
          in: query
          description: Trader PDA index to scope history. Defaults to 0.
          required: false
          schema:
            type: integer
            format: int32
          example: 0
        - name: subaccountIndex
          in: query
          description: >-
            Optional subaccount index. If omitted, all subaccounts under the PDA
            are

            included.
          required: false
          schema:
            type:
              - integer
              - 'null'
            format: int32
        - name: symbol
          in: query
          description: Optional market symbol filter.
          required: false
          schema:
            type:
              - string
              - 'null'
        - name: limit
          in: query
          description: Maximum number of events to return (max 100, default 100).
          required: false
          schema:
            type: integer
            format: int64
          example: 100
        - name: cursor
          in: query
          description: Opaque cursor for older or newer pagination.
          required: false
          schema:
            type:
              - string
              - 'null'
        - name: user_pubkey
          in: path
          required: true
          schema:
            type: string
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                $ref: >-
                  #/components/schemas/PaginatedResponse_Vec_UserLiquidationHistoryPoint
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    PaginatedResponse_Vec_UserLiquidationHistoryPoint:
      type: object
      description: >-
        Generic paginated response wrapper with bidirectional cursor support.


        The cursor system supports both forward (newer) and backward (older)

        pagination:

        - `prev_cursor`: Use this cursor to poll for new items (items newer than
        the
          current result set)
        - `next_cursor`: Use this cursor to load more items (items older than
        the
          current result set)

        The direction is embedded in the cursor itself, so clients just need to
        pass

        the appropriate cursor to the `cursor` parameter.
      required:
        - data
        - hasMore
      properties:
        data:
          type: array
          items:
            type: object
            description: >-
              A normalized liquidation, backstop, or ADL event for one user
              authority.
            required:
              - kind
              - type
              - ixName
              - role
              - slot
              - slotIndex
              - eventIndex
              - timestamp
              - symbol
              - market
            properties:
              atLossCloseValue:
                type:
                  - string
                  - 'null'
              atLossCollateralChange:
                type:
                  - string
                  - 'null'
              baseLotsFilled:
                type:
                  - string
                  - 'null'
              caller:
                type:
                  - string
                  - 'null'
              closedLong:
                type:
                  - string
                  - 'null'
              closedShort:
                type:
                  - string
                  - 'null'
              eventIndex:
                type: integer
                format: int32
              haircutRateBps:
                type:
                  - integer
                  - 'null'
                format: int32
              inProfitAccount:
                type:
                  - string
                  - 'null'
              inProfitCloseValue:
                type:
                  - string
                  - 'null'
              inProfitCollateralChange:
                type:
                  - string
                  - 'null'
              ixName:
                type: string
                description: Indexed instruction name that emitted this event.
              kind:
                $ref: '#/components/schemas/UserLiquidationHistoryKind'
              liquidatee:
                type:
                  - string
                  - 'null'
              liquidateeCollateralChange:
                type:
                  - string
                  - 'null'
              liquidator:
                type:
                  - string
                  - 'null'
              liquidatorCollateralChange:
                type:
                  - string
                  - 'null'
              market:
                type: string
                description: >-
                  Alias for `symbol`, included for clients that present this
                  field as the

                  market.
              positionClosed:
                type:
                  - boolean
                  - 'null'
              price:
                type:
                  - string
                  - 'null'
              quoteLotsFilled:
                type:
                  - string
                  - 'null'
              quoteSize:
                type:
                  - string
                  - 'null'
              role:
                $ref: '#/components/schemas/UserLiquidationHistoryRole'
              side:
                type:
                  - string
                  - 'null'
              signature:
                type:
                  - string
                  - 'null'
              size:
                type:
                  - string
                  - 'null'
              slot:
                type: integer
                format: int64
              slotIndex:
                type: integer
                format: int32
              subaccountIndex:
                type:
                  - integer
                  - 'null'
                format: int32
              symbol:
                type: string
                description: Market symbol, for example `SOL-PERP`.
              timestamp:
                type: integer
                format: int64
              type:
                $ref: '#/components/schemas/UserLiquidationHistoryType'
                description: 'High-level event type: `market`, `adl`, or `backstop`.'
        hasMore:
          type: boolean
          description: Whether there are more results available after this page
        nextCursor:
          type:
            - string
            - 'null'
          description: >-
            Opaque cursor for fetching the next page of older results.

            Pass this value as the `cursor` parameter in the next request to
            load

            more.
        prevCursor:
          type:
            - string
            - 'null'
          description: >-
            Opaque cursor for fetching newer items (for polling).

            Pass this value as the `cursor` parameter to get items newer than
            the

            first item in data.
    UserLiquidationHistoryKind:
      type: string
      description: User-scoped liquidation history event kind.
      enum:
        - market_order
        - adl
        - backstop
    UserLiquidationHistoryRole:
      type: string
      description: Role the requested user played in a liquidation-related event.
      enum:
        - liquidatee
        - backstop_liquidatee
        - adl_closed_short
        - adl_closed_long
        - adl_in_profit
        - adl_caller
    UserLiquidationHistoryType:
      type: string
      description: High-level liquidation-related event type.
      enum:
        - market
        - adl
        - backstop
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/get-user-liquidation-history.md -->

---

# Source: `api/trader/get-user-pnl.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/get-user-pnl.md`

**Local path:** `api/trader/get-user-pnl.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get user PnL

> Handles `GET /v1/users/{user_pubkey}/pnl` via `get.v1.users.by_user_pubkey.pnl`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/users/{user_pubkey}/pnl
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/users/{user_pubkey}/pnl:
    get:
      tags:
        - Trader
      summary: Get user PnL
      description: >-
        Handles `GET /v1/users/{user_pubkey}/pnl` via
        `get.v1.users.by_user_pubkey.pnl`.
      operationId: get.v1.users.by_user_pubkey.pnl
      parameters:
        - name: user_pubkey
          in: path
          description: User public key (base58 encoded)
          required: true
          schema:
            type: string
        - name: resolution
          in: query
          description: 'Resolution/timeframe: 1m, 1d'
          required: true
          schema:
            type: string
        - name: startTime
          in: query
          description: 'Start time in milliseconds since Unix epoch (max range: 1 year)'
          required: false
          schema:
            type: integer
            format: int64
        - name: endTime
          in: query
          description: 'End time in milliseconds since Unix epoch (max range: 1 year)'
          required: false
          schema:
            type: integer
            format: int64
        - name: limit
          in: query
          description: 'Max number of data points (default: 1000, max: 1440)'
          required: false
          schema:
            type: integer
            format: int64
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/PnLPoint'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    PnLPoint:
      type: object
      description: PnL data point
      required:
        - timestamp
        - startTime
        - endTime
        - cumulativePnl
        - unrealizedPnl
        - cumulativeFundingPayment
        - cumulativeTakerFee
      properties:
        cumulativeFundingPayment:
          type: number
          format: double
          description: Cumulative funding payment up to this point
        cumulativePnl:
          type: number
          format: double
          description: Cumulative realized PnL up to this point
        cumulativeTakerFee:
          type: number
          format: double
          description: Cumulative taker fees paid up to this point
        endTime:
          type: integer
          format: int64
          description: End time in seconds since Unix epoch
        startTime:
          type: integer
          format: int64
          description: Start time in seconds since Unix epoch
        timestamp:
          type: integer
          format: int64
          description: 'Deprecated: Unix timestamp in seconds.'
        unrealizedPnl:
          type: number
          format: double
          description: unrealized PnL up to this point
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/get-user-pnl.md -->

---

# Source: `api/trader/get-user-trade-history-v2.md`

**Original URL:** `https://docs.phoenix.trade/api/trader/get-user-trade-history-v2.md`

**Local path:** `api/trader/get-user-trade-history-v2.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Get user trade history v2

> Handles `GET /v1/users/{user_pubkey}/trades_v2` via `get.v1.users.by_user_pubkey.trades_v2`.



## OpenAPI

````yaml /openapi/phoenix-public-api.json get /v1/users/{user_pubkey}/trades_v2
openapi: 3.1.0
info:
  title: Phoenix Eternal API
  description: >-
    RESTful API for accessing Phoenix Eternal perpetual trading data. Provides
    real-time orderbook levels, asset information, and market metadata.
  termsOfService: https://www.phoenix.trade/terms-of-service
  contact:
    name: Phoenix Development Team
    url: https://github.com/Ellipsis-Labs/phoenix
  license:
    name: Proprietary
  version: 1.0.0
servers:
  - url: https://perp-api.phoenix.trade
    description: Phoenix Eternal Perp API
security: []
tags:
  - name: Auth
    description: Wallet and session authentication.
  - name: Exchange
    description: Exchange, market, candle, and no-referral register-instruction workflows.
  - name: Invite
    description: Invite validation, referral activation, and wallet allowlist checks.
  - name: Notifications
    description: Trader notification reads and acknowledgement helpers.
  - name: Trader
    description: Trader state, account history, and transaction builders.
externalDocs:
  url: https://docs.phoenix.trade
  description: Phoenix developer documentation
paths:
  /v1/users/{user_pubkey}/trades_v2:
    get:
      tags:
        - Trader
      summary: Get user trade history v2
      description: >-
        Handles `GET /v1/users/{user_pubkey}/trades_v2` via
        `get.v1.users.by_user_pubkey.trades_v2`.
      operationId: get.v1.users.by_user_pubkey.trades_v2
      parameters:
        - name: market_symbol
          in: query
          description: Optional market symbol to filter by (e.g., "SOL-PERP")
          required: false
          schema:
            type: string
        - name: trader_pda_index
          in: query
          description: Optional trader PDA index to scope results for a specific trader PDA
          required: false
          schema:
            type: integer
            format: int32
        - name: limit
          in: query
          description: Number of items to return (max 1000)
          required: false
          schema:
            type: integer
            format: int64
        - name: cursor
          in: query
          description: |-
            Optional cursor for pagination (format: "slot" or "slot,slot_index")
            Items returned will be older than (exclusive of) this cursor
          required: false
          schema:
            type: string
        - name: privy_id
          in: query
          description: Privy user ID (optional)
          required: false
          schema:
            type: string
        - name: user_pubkey
          in: path
          required: true
          schema:
            type: string
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/PaginatedResponse_Vec_TradeHistoryV2Item'
      security:
        - PhoenixBearerAuth: []
components:
  schemas:
    PaginatedResponse_Vec_TradeHistoryV2Item:
      type: object
      description: >-
        Generic paginated response wrapper with bidirectional cursor support.


        The cursor system supports both forward (newer) and backward (older)

        pagination:

        - `prev_cursor`: Use this cursor to poll for new items (items newer than
        the
          current result set)
        - `next_cursor`: Use this cursor to load more items (items older than
        the
          current result set)

        The direction is embedded in the cursor itself, so clients just need to
        pass

        the appropriate cursor to the `cursor` parameter.
      required:
        - data
        - hasMore
      properties:
        data:
          type: array
          items:
            type: object
            description: A single flattened trade history item with PnL and position data
            required:
              - userId
              - traderId
              - traderPdaIndex
              - subaccountIndex
              - marketSymbol
              - timestamp
              - slot
              - slotIndex
              - eventIndex
              - instructionIndex
              - instructionType
              - baseLotsBefore
              - baseLotsAfter
              - baseLotsDelta
              - virtualQuoteLotsBefore
              - virtualQuoteLotsAfter
              - virtualQuoteLotsDelta
              - price
              - realizedPnl
              - fees
              - liquidity
              - tradeType
            properties:
              baseLotsAfter:
                type: string
                description: Base lots after the trade (human readable)
              baseLotsBefore:
                type: string
                description: Base lots before the trade (human readable)
              baseLotsDelta:
                type: string
                description: Base lots delta (human readable, signed)
              eventIndex:
                type: integer
                format: int32
              fees:
                type: string
              fillId:
                type:
                  - string
                  - 'null'
                description: Deterministic UUID v3 derived from the raw fill coordinates.
              instructionIndex:
                type: integer
                format: int32
              instructionType:
                type: string
                description: Instruction type (e.g., "PlaceLimitOrder", "PlaceMarketOrder")
              liquidity:
                $ref: '#/components/schemas/LiquidityRole'
              marketSymbol:
                type: string
                description: Market symbol
              orderSequenceNumber:
                type:
                  - integer
                  - 'null'
                format: int64
              price:
                type: string
              realizedPnl:
                type: string
                description: Realized PnL from this trade (human readable USD)
              signature:
                type:
                  - string
                  - 'null'
                description: Transaction signature
              slot:
                type: integer
                format: int64
                description: Slot coordinates for cursor
              slotIndex:
                type: integer
                format: int32
              splineSequenceNumber:
                type:
                  - integer
                  - 'null'
                format: int64
              subaccountIndex:
                type: integer
                format: int32
                description: Subaccount index
              timestamp:
                type: string
                format: date-time
                description: Formatted datetime string (ISO 8601).
              tradeType:
                $ref: '#/components/schemas/TradeType'
              traderId:
                type: integer
                format: int64
                description: Trader ID
              traderPdaIndex:
                type: integer
                format: int32
                description: Trader PDA index
              userId:
                type: integer
                format: int64
                description: User ID
              virtualQuoteLotsAfter:
                type: string
                description: Virtual quote lots after (human readable)
              virtualQuoteLotsBefore:
                type: string
                description: Virtual quote lots before (human readable)
              virtualQuoteLotsDelta:
                type: string
                description: Virtual quote lots delta (human readable, signed)
        hasMore:
          type: boolean
          description: Whether there are more results available after this page
        nextCursor:
          type:
            - string
            - 'null'
          description: >-
            Opaque cursor for fetching the next page of older results.

            Pass this value as the `cursor` parameter in the next request to
            load

            more.
        prevCursor:
          type:
            - string
            - 'null'
          description: >-
            Opaque cursor for fetching newer items (for polling).

            Pass this value as the `cursor` parameter to get items newer than
            the

            first item in data.
    LiquidityRole:
      type: string
      description: Liquidity role for the tracked account's fill.
      enum:
        - maker
        - taker
    TradeType:
      type: string
      enum:
        - limit
        - market
        - liquidation
        - adl
  securitySchemes:
    PhoenixBearerAuth:
      type: http
      scheme: bearer
      description: Bearer access token issued by `/v1/auth/*` login endpoints.

````

<!-- END SOURCE: api/trader/get-user-trade-history-v2.md -->

---

# Source: `api/websocket.md`

**Original URL:** `https://docs.phoenix.trade/api/websocket.md`

**Local path:** `api/websocket.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# WebSocket

> Supported Phoenix WebSocket subscription requests and response message types.

## Connection

Connect to the Phoenix WebSocket endpoint:

```text theme={null}
wss://perp-api.phoenix.trade/v1/ws
```

The Rust SDK uses `PHOENIX_WS_URL` when it is set. Otherwise, it derives the WebSocket URL from `PHOENIX_API_URL` by switching `http` to `ws` and using `/v1/ws`.

Messages are UTF-8 JSON objects.

## Client messages

Subscribe with a `type` and `subscription` envelope:

```json theme={null}
{
  "type": "subscribe",
  "subscription": {
    "channel": "orderbook",
    "symbol": "SOL"
  }
}
```

Unsubscribe with the same subscription object:

```json theme={null}
{
  "type": "unsubscribe",
  "subscription": {
    "channel": "orderbook",
    "symbol": "SOL"
  }
}
```

## Supported channels

| Channel       | Subscription fields                      | Response type              |
| ------------- | ---------------------------------------- | -------------------------- |
| `allMids`     | none                                     | `AllMidsData`              |
| `exchange`    | optional `encoding`                      | `ExchangeMessage`          |
| `fundingRate` | `symbol`                                 | `FundingRateMessage`       |
| `orderbook`   | `symbol`, optional `bypassExecutionBand` | `L2BookUpdate`             |
| `traderState` | `authority`, `traderPdaIndex`            | `TraderStateServerMessage` |
| `market`      | `symbol`                                 | `MarketStatsUpdate`        |
| `trades`      | `symbol`                                 | `TradesMessage`            |
| `candles`     | `symbol`, `timeframe`                    | `CandleData`               |

## Control responses

The server confirms successful subscriptions with a `subscriptionConfirmed` message:

```json theme={null}
{
  "type": "subscriptionConfirmed",
  "subscription": {
    "channel": "orderbook",
    "symbol": "SOL"
  }
}
```

Subscription-level errors use `subscriptionError`:

```json theme={null}
{
  "type": "subscriptionError",
  "subscription": {
    "channel": "orderbook",
    "symbol": "SOL"
  },
  "code": "invalid_subscription",
  "message": "invalid subscription"
}
```

Server errors use the `error` channel:

```json theme={null}
{
  "channel": "error",
  "code": 400,
  "error": "unknown channel"
}
```

## Market subscriptions

### Exchange

Use the `exchange` channel to keep local exchange and market-parameter metadata in sync. The server sends an initial snapshot followed by ordered deltas when exchange keys, exchange status, markets, or market parameters change.

```json theme={null}
{
  "type": "subscribe",
  "subscription": {
    "channel": "exchange",
    "encoding": "json"
  }
}
```

The `encoding` field is optional. Omit it to use the default compressed snapshot response:

```json theme={null}
{
  "channel": "exchange",
  "messageType": "encodedSnapshot",
  "version": 1,
  "sequenceNumber": "123",
  "slot": 123456789,
  "slotIndex": 0,
  "reason": "snapshot",
  "encoding": "base64+zstd",
  "payload": "..."
}
```

Set `encoding` to `json` to receive the initial snapshot as JSON. The examples below are abbreviated to show the fields relevant to cache synchronization:

```json theme={null}
{
  "channel": "exchange",
  "messageType": "snapshot",
  "version": 1,
  "sequenceNumber": "123",
  "slot": 123456789,
  "slotIndex": 0,
  "reason": "snapshot",
  "exchange": {
    "programId": "program-pubkey",
    "globalConfig": "global-config-pubkey",
    "active": true,
    "gated": false
  },
  "markets": [
    {
      "symbol": "SOL",
      "assetId": 1,
      "marketStatus": "active",
      "marketPubkey": "market-pubkey",
      "splinePubkey": "spline-pubkey"
    }
  ]
}
```

After the snapshot, apply each `delta` with the next `sequenceNumber` to your local cache. If a sequence number is skipped, resubscribe and rebuild from a fresh snapshot.

```json theme={null}
{
  "channel": "exchange",
  "messageType": "delta",
  "version": 1,
  "sequenceNumber": "124",
  "slot": 123456790,
  "slotIndex": 0,
  "ops": [
    {
      "kind": "marketAdded",
      "market": {
        "symbol": "ETH",
        "assetId": 2,
        "marketStatus": "active",
        "marketPubkey": "market-pubkey",
        "splinePubkey": "spline-pubkey"
      }
    },
    {
      "kind": "marketClosed",
      "symbol": "SOL",
      "previous_market_status": "active",
      "finalized_mark_price": "150000000"
    }
  ]
}
```

Handle exchange deltas as cache mutations:

| Delta kind               | Sync behavior                                                                                            |
| ------------------------ | -------------------------------------------------------------------------------------------------------- |
| `marketAdded`            | Add or replace the market config in the local market map.                                                |
| `marketStatusChanged`    | Update the market's `marketStatus`.                                                                      |
| `marketClosed`           | Mark the market as `closed` and retain the finalized mark price if your client needs settlement context. |
| `marketTombstoned`       | Mark the market as `tombstoned` and stop treating it as tradeable.                                       |
| `marketDeleted`          | Remove the market from the local market map.                                                             |
| `marketParameterUpdated` | Patch the market's risk, funding, fee, cap, or commodity metadata fields from the update payload.        |
| `exchangeStatusChanged`  | Update exchange-level `active`, `gated`, and status feature flags.                                       |
| `exchangeKeysUpdated`    | Replace exchange-level account and authority keys.                                                       |

### All mids

```json theme={null}
{
  "type": "subscribe",
  "subscription": {
    "channel": "allMids"
  }
}
```

```json theme={null}
{
  "channel": "allMids",
  "mids": {
    "SOL": 150.25,
    "BTC": 65000.5
  },
  "slot": 123456789,
  "slotIndex": 0
}
```

### Funding rate

```json theme={null}
{
  "type": "subscribe",
  "subscription": {
    "channel": "fundingRate",
    "symbol": "SOL"
  }
}
```

```json theme={null}
{
  "channel": "fundingRate",
  "symbol": "SOL",
  "funding": 0.0000125
}
```

### Orderbook

Set `bypassExecutionBand` to `true` to receive the full orderbook during commodities after-hours, including price levels outside the tradeable execution band.

```json theme={null}
{
  "type": "subscribe",
  "subscription": {
    "channel": "orderbook",
    "symbol": "SOL",
    "bypassExecutionBand": false
  }
}
```

```json theme={null}
{
  "channel": "orderbook",
  "symbol": "SOL",
  "orderbook": {
    "bids": [[150.25, 100.0], [150.2, 200.0]],
    "asks": [[150.3, 150.0], [150.35, 250.0]],
    "mid": 150.275
  },
  "bypassExecutionBand": false
}
```

### Market stats

```json theme={null}
{
  "type": "subscribe",
  "subscription": {
    "channel": "market",
    "symbol": "SOL"
  }
}
```

```json theme={null}
{
  "channel": "market",
  "symbol": "SOL",
  "openInterest": 1000.0,
  "markPx": 150.1,
  "midPx": 150.12,
  "oraclePx": 150.0,
  "prevDayPx": 148.7,
  "dayNtlVlm": 2500000.0,
  "funding": 0.0000125
}
```

### Trades

```json theme={null}
{
  "type": "subscribe",
  "subscription": {
    "channel": "trades",
    "symbol": "SOL"
  }
}
```

```json theme={null}
{
  "channel": "trades",
  "symbol": "SOL",
  "trades": [
    {
      "slot": "123456789",
      "slotIndex": 5,
      "timestamp": "1775578550",
      "symbol": "SOL",
      "taker": "taker-pubkey",
      "tradeSequenceNumber": "100",
      "side": "bid",
      "baseLotsFilled": "1000",
      "quoteLotsFilled": "150000",
      "feeInQuoteLots": "30",
      "baseAmount": 10.0,
      "quoteAmount": 1500.0,
      "numFills": 2
    }
  ]
}
```

### Candles

Supported timeframes are `1s`, `5s`, `1m`, `5m`, `15m`, `30m`, `1h`, `4h`, and `1d`.

```json theme={null}
{
  "type": "subscribe",
  "subscription": {
    "channel": "candles",
    "symbol": "SOL",
    "timeframe": "1m"
  }
}
```

The server sends candle updates with `channel` set to `candle`:

```json theme={null}
{
  "channel": "candle",
  "symbol": "SOL",
  "timeframe": "1m",
  "candle": {
    "time": 1727181985,
    "low": 149.8,
    "high": 151.5,
    "open": 150.25,
    "close": 150.9,
    "volume": 1234.56,
    "tradeCount": 89
  }
}
```

## Trader subscriptions

### Trader state

```json theme={null}
{
  "type": "subscribe",
  "subscription": {
    "channel": "traderState",
    "authority": "wallet-pubkey",
    "traderPdaIndex": 0
  }
}
```

Trader state messages are either `snapshot` or `delta` payloads.

```json theme={null}
{
  "channel": "traderState",
  "authority": "wallet-pubkey",
  "traderPdaIndex": 0,
  "slot": 123456789,
  "messageType": "snapshot",
  "version": 1,
  "capabilities": {
    "flags": 63,
    "state": "active",
    "capabilities": {
      "placeLimitOrder": { "immediate": true },
      "placeMarketOrder": { "immediate": true },
      "riskIncreasingTrade": { "immediate": true },
      "riskReducingTrade": { "immediate": true },
      "depositCollateral": { "immediate": true },
      "withdrawCollateral": { "immediate": true }
    }
  },
  "makerFeeOverrideMultiplier": 1.0,
  "takerFeeOverrideMultiplier": 1.0,
  "subaccounts": []
}
```

```json theme={null}
{
  "channel": "traderState",
  "authority": "wallet-pubkey",
  "traderPdaIndex": 0,
  "slot": 123456790,
  "messageType": "delta",
  "deltas": [
    {
      "subaccountIndex": 0,
      "sequence": 42,
      "collateral": "1000000",
      "positions": [],
      "orders": [],
      "splines": [],
      "tradeHistory": [],
      "orderHistory": []
    }
  ]
}
```

<!-- END SOURCE: api/websocket.md -->

---

# Source: `builder-codes.md`

**Original URL:** `https://docs.phoenix.trade/builder-codes.md`

**Local path:** `builder-codes.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Overview

> How Flight codes work on Phoenix

> Builder codes for Phoenix — earn a share of fees on the order flow you route.

Flight is Phoenix's builder-code layer. If you run a trading app, terminal, bot, or agent that sends orders to Phoenix, you can register as a builder, route orders through
Flight, and collect a fee on the volume you bring.

## How it works

1. Register a builder at [flight dashboard](https://flight.phoenix.trade). Registration is an on-chain instruction that ties your wallet (the builder authority) to a Phoenix trader account where Flight
   fees accrue.
2. Set your builder fee in basis points at registration. The fee is added on top of Phoenix's base exchange fees and is paid by the trader on each routed order.
3. Route orders through Flight from your client. Once the SDK client is configured with your builder authority, supported order instructions are wrapped automatically. See the [SDK
   docs](/sdk/rise) for installation and client setup.
4. Collect fees as collateral on your builder trader account. Withdraw any time from the Flight portal.

## Fees

Flight currently collects builder fees on liquidity-removing fills:

* Market orders
* The taking portion of a limit order that crosses the book

If a routed limit order rests as maker liquidity, the resting portion does not generate a Flight fee today. Maker-side fee collection is on the roadmap so builders can also earn on
routed liquidity that rests first and fills later.

The builder fee is separate from — and stacks on top of — Phoenix's [base taker and maker fees](/phoenix/matching-engine/fees). Builders set their own bps at registration.

## Registering

Register using the [flight dashboard](https://flight.phoenix.trade). The portal is also where you track accrued fees and withdraw funds.

We recommend registering with a fresh, empty wallet. Builder fees accrue as collateral on the Phoenix trader account tied to your builder authority, so a clean wallet keeps builder
revenue isolated from any existing trading collateral.

## Using Flight

We recommend creating a **dedicated embedded wallet** for each user who interacts with Phoenix. This keeps their Phoenix activity isolated from other platforms that integrate with Phoenix.

## Onboarding users to Phoenix

Flight builders and other integrations can onboard users without referral codes through `POST /v1/exchange/build-register-ixs` followed by `POST /v1/exchange/send-register-ixs`. Your app builds and signs the transaction locally with the user's trader authority and a non-Phoenix fee payer, then the API validates, signs with the Phoenix onboarder, simulates to ensure Phoenix pays no lamports, sends the transaction, and returns the signature. See [Trader Onboarding](/sdk/register#without-a-referral-code).

## SDK integration

Rise, the Phoenix SDK, supports Flight-routed order instructions in TypeScript and Rust. See the [SDK docs](/sdk/rise) to install the SDK and create a client.

> Flight support in Rise is currently in beta.

## See also

* [Fees](/phoenix/matching-engine/fees)
* [Referral Program](/incentive-programs/referral-program)
* [SDK docs](/sdk/rise)
* [FAQ](/phoenix/faq/faq)

<!-- END SOURCE: builder-codes.md -->

---

# Source: `cli/commands.md`

**Original URL:** `https://docs.phoenix.trade/cli/commands.md`

**Local path:** `cli/commands.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Command reference

> Every Vulcan CLI command, grouped by topic.

This reference lists every command group Vulcan exposes. For strategy runners (TWAP and grid), see the dedicated [Strategies](/cli/strategies) page.

## Global flags

These flags are available on every command:

| Flag                    | Description                                              |
| ----------------------- | -------------------------------------------------------- |
| `-o, --output <format>` | Output format: `table` (default) or `json`.              |
| `--dry-run`             | Simulate the operation without submitting a transaction. |
| `-y, --yes`             | Skip confirmation prompts.                               |
| `-w, --wallet <name>`   | Stored wallet to use instead of the default.             |
| `--rpc-url <url>`       | Solana RPC endpoint override.                            |
| `--api-url <url>`       | Phoenix API endpoint override.                           |
| `-v, --verbose`         | Enable verbose / debug logging to stderr.                |
| `--watch`               | Watch for live updates via WebSocket where supported.    |

## `wallet` — wallet management

| Subcommand                                                                                                  | Description                                                                                                                                                                                                                                                                                                                         |
| ----------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `create --name <name>`                                                                                      | Generate a new Solana keypair, encrypt it, and store it.                                                                                                                                                                                                                                                                            |
| `import --name <name> [--format base58\|bytes\|file] <source>`                                              | Import from base58 string, byte array, or Solana CLI JSON file.                                                                                                                                                                                                                                                                     |
| `list`                                                                                                      | List all stored wallets.                                                                                                                                                                                                                                                                                                            |
| `show <name>`                                                                                               | Show wallet details (pubkey, default status).                                                                                                                                                                                                                                                                                       |
| `set-default <name>`                                                                                        | Set a wallet as the default for all commands.                                                                                                                                                                                                                                                                                       |
| `remove <name>`                                                                                             | Remove a wallet from local storage.                                                                                                                                                                                                                                                                                                 |
| `export <name> [--file <path> \| --stdout \| --private-key [--private-key-format base58\|bytes]] [--force]` | Export the encrypted wallet file to `<path>` (`--file`), print the encrypted backup to stdout (`--stdout`), or print the plaintext private key (`--private-key`, requires `--yes`). `--private-key-format` defaults to `base58`; pass `bytes` for the Solana CLI JSON array format. `--force` overwrites an existing `--file` path. |
| `balance [<name>]`                                                                                          | Show SOL and USDC balances. Defaults to the default wallet.                                                                                                                                                                                                                                                                         |

```bash theme={null}
vulcan wallet create --name my-wallet
vulcan wallet balance my-wallet -o json
```

## `market` — market data

| Subcommand                                                                   | Description                                                                                                                                                                       |
| ---------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `list`                                                                       | List all available perpetual markets.                                                                                                                                             |
| `info <symbol>`                                                              | Detailed market configuration (tick size, lot size, fees, leverage tiers).                                                                                                        |
| `ticker <symbol>`                                                            | Current price, 24h volume, open interest, funding rate.                                                                                                                           |
| `orderbook <symbol> [--depth <n>]`                                           | L2 orderbook snapshot. Default depth 10.                                                                                                                                          |
| `candles <symbol> [--interval <i>] [--limit <n>] [--with-indicators <list>]` | OHLCV candles. Defaults: interval `1h`, limit `20`. `--with-indicators` appends derived columns; supported: `sma`, `ema`, `rsi`, `macd`, `bbands`, `atr`, `vwap`, `adx`, `stoch`. |
| `trades <symbol> [--limit <n>]`                                              | Recent trades. Default limit 20.                                                                                                                                                  |
| `funding-rates <symbol> [--limit <n>]`                                       | Historical funding rates. Default limit 20.                                                                                                                                       |

```bash theme={null}
vulcan market ticker SOL -o json
vulcan market orderbook SOL --depth 5
```

## `trade` — order management

| Subcommand                                                                                                                           | Description                                                                                               |
| ------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------- |
| `market-buy <symbol> [<size> \| --tokens <t> \| --notional-usdc <u>] [--tp] [--sl] [--isolated [--collateral <c>]] [--reduce-only]`  | Place a market buy. Provide exactly one of positional size (base lots), `--tokens`, or `--notional-usdc`. |
| `market-sell <symbol> [<size> \| --tokens <t> \| --notional-usdc <u>] [--tp] [--sl] [--isolated [--collateral <c>]] [--reduce-only]` | Place a market sell. Same sizing options as `market-buy`.                                                 |
| `limit-buy <symbol> <size> <price> [--tp] [--sl] [--isolated [--collateral <c>]] [--reduce-only]`                                    | Place a limit buy. Size in base lots.                                                                     |
| `limit-sell <symbol> <size> <price> [--tp] [--sl] [--isolated [--collateral <c>]] [--reduce-only]`                                   | Place a limit sell. Size in base lots.                                                                    |
| `cancel <symbol> <order-id...>`                                                                                                      | Cancel specific orders by ID.                                                                             |
| `cancel-all [<symbol>]`                                                                                                              | Cancel all open orders. Omit symbol to cancel across every market.                                        |
| `orders [<symbol>]`                                                                                                                  | List open orders. Omit symbol to list all.                                                                |
| `set-tpsl <symbol> [--tp \| --tp-level PRICE[:SIZE_TOKENS] ...] [--sl \| --sl-level PRICE[:SIZE_TOKENS] ...]`                        | Set take-profit or stop-loss on an existing position. Supports laddered exits.                            |
| `cancel-tpsl <symbol> [--tp] [--sl]`                                                                                                 | Cancel take-profit and/or stop-loss for a position.                                                       |

```bash theme={null}
vulcan trade market-buy SOL --notional-usdc 100 --tp 250 --sl 180
vulcan trade limit-sell SOL 1.5 230 --reduce-only
vulcan trade cancel-all
```

## `position` — position management

| Subcommand                                     | Description                                                  |
| ---------------------------------------------- | ------------------------------------------------------------ |
| `list`                                         | List all open positions.                                     |
| `show <symbol>`                                | Detailed view of a specific position.                        |
| `close <symbol>`                               | Close an entire position.                                    |
| `close-all`                                    | Close every open position across all markets.                |
| `reduce <symbol> <size>`                       | Reduce a position by `size` base lots.                       |
| `tp-sl <symbol> [--tp <price>] [--sl <price>]` | Attach take-profit and/or stop-loss to an existing position. |

```bash theme={null}
vulcan position list -o json
vulcan position close SOL
```

## `margin` — collateral management

| Subcommand                                  | Description                                                              |
| ------------------------------------------- | ------------------------------------------------------------------------ |
| `status`                                    | Show cross-margin health, equity, maintenance margin, available balance. |
| `deposit <amount>`                          | Deposit USDC collateral.                                                 |
| `withdraw <amount>`                         | Withdraw USDC collateral.                                                |
| `transfer <amount> --from <idx> --to <idx>` | Transfer collateral between subaccounts (0 = cross).                     |
| `transfer-child-to-parent --child <idx>`    | Sweep all collateral from a child subaccount back to cross-margin.       |
| `sync-parent-to-child --child <idx>`        | Sync parent state to a child subaccount.                                 |
| `leverage-tiers <symbol>`                   | Show the leverage tier schedule for a market.                            |
| `add-collateral <symbol> <amount>`          | Add USDC to an isolated position.                                        |

```bash theme={null}
vulcan margin status -o json
vulcan margin deposit 500
```

## `account` — trader account

| Subcommand                                                                        | Description                                                  |
| --------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| `register --access-code <code> \| --referral-code <code> \| --invite-code <code>` | Register a trader account. Exactly one code is required.     |
| `info`                                                                            | Show trader account details (PDA, subaccounts, margin mode). |
| `subaccounts`                                                                     | List all subaccounts.                                        |
| `create-subaccount [--pda-index <n>] --subaccount-index <n>`                      | Create a new subaccount.                                     |

```bash theme={null}
vulcan account register --access-code MY_CODE
vulcan account info -o json
```

## `auth` — Phoenix API authentication

| Subcommand | Description                                              |
| ---------- | -------------------------------------------------------- |
| `login`    | Log in to the Phoenix API by signing a wallet challenge. |
| `status`   | Show redacted Phoenix API session status.                |
| `logout`   | Clear the stored Phoenix API session.                    |

```bash theme={null}
vulcan auth login
vulcan auth status -o json
```

## `portfolio` — portfolio snapshot

A single command (no subcommands) that returns margin, positions, and open orders in one call.

| Flag                   | Description                                                                 |
| ---------------------- | --------------------------------------------------------------------------- |
| `--include <sections>` | Comma-separated subset of `margin`, `positions`, `orders`. Defaults to all. |

```bash theme={null}
vulcan portfolio -o json
vulcan portfolio --include margin,positions
```

## `paper` — local paper trading

Paper trading runs against live Phoenix prices but never touches your wallet or on-chain state.

| Subcommand                                                                                                  | Description                                                      |
| ----------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| `init [--balance <amount>] [--currency <code>] [--fee-bps <bps>]`                                           | Initialize or overwrite the local paper account.                 |
| `reset [--balance <amount>] [--currency <code>] [--fee-bps <bps>]`                                          | Reset local paper state.                                         |
| `status`                                                                                                    | Show paper account status.                                       |
| `positions`                                                                                                 | Show paper positions.                                            |
| `orders`                                                                                                    | Show open paper orders.                                          |
| `fills [--limit <n>]`                                                                                       | Show recent paper fills. Default limit 50.                       |
| `buy <symbol> [--type market\|limit] [--size <lots> \| --tokens <t> \| --notional-usdc <u>] [--price <p>]`  | Place a paper buy order.                                         |
| `sell <symbol> [--type market\|limit] [--size <lots> \| --tokens <t> \| --notional-usdc <u>] [--price <p>]` | Place a paper sell order.                                        |
| `cancel <order-id>`                                                                                         | Cancel a paper order.                                            |
| `cancel-all [<symbol>]`                                                                                     | Cancel all paper orders, optionally filtered by symbol.          |
| `reconcile [<symbol>]`                                                                                      | Reconcile resting paper limit orders against live market prices. |

```bash theme={null}
vulcan paper init --balance 10000
vulcan paper buy SOL --notional-usdc 100 --type market -o json
```

## `history` — trade and account history

| Subcommand                                            | Description                                       |
| ----------------------------------------------------- | ------------------------------------------------- |
| `trades [--symbol <s>] [--limit <n>] [--cursor <c>]`  | Past trade and fill history. Default limit 20.    |
| `orders [--symbol <s>] [--limit <n>] [--cursor <c>]`  | Past order history. Default limit 20.             |
| `collateral [--limit <n>] [--cursor <c>]`             | Deposit and withdrawal history. Default limit 20. |
| `funding [--symbol <s>] [--limit <n>] [--cursor <c>]` | Funding payment history. Default limit 20.        |
| `pnl [--resolution hourly\|daily] [--limit <n>]`      | PnL over time. Defaults to `hourly`, limit 24.    |

```bash theme={null}
vulcan history trades --symbol SOL --limit 50
vulcan history pnl --resolution daily --limit 30
```

## `agent` — agent setup

| Subcommand                                                                                                       | Description                                                                                                                                |
| ---------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `install [--target <t>] [--scope user\|project] [--dir <path>] [--force]`                                        | Install Vulcan agent skills for a client. Targets: `claude`, `cursor`, `codex`, `agentskills`.                                             |
| `doctor [--target <t>] [--scope user\|project] [--dir <path>]`                                                   | Inspect known agent skill locations.                                                                                                       |
| `health [--target <t>] [--scope user\|project] [--dir <path>]`                                                   | Combined health check for first-run agent guidance.                                                                                        |
| `live-ready [--target <t>] [--scope user\|project]`                                                              | Check whether live agent execution is ready for a target client.                                                                           |
| `mcp print-config [--target <t>] [--dangerous] [--groups <list>]`                                                | Print an MCP config snippet for an agent client.                                                                                           |
| `mcp doctor [--target <t>] [--scope <s>] [--path <path>]`                                                        | Inspect expected MCP config for an agent client.                                                                                           |
| `mcp install [--target <t>] [--scope <s>] [--path <path>] [--dangerous] [--groups <list>] [--force \| --repair]` | Install or update the Vulcan MCP config. `--repair` updates only `command` and `args`, preserving the env (wallet name and password).      |
| `mcp set-wallet <wallet> [--target <t>] [--scope <s>] [--path <path>]`                                           | Switch the wallet used by an already-installed Vulcan MCP server.                                                                          |
| `mcp diagnose [--target <t>] [--scope <s>] [--path <path>]`                                                      | Spawn the server with the exact command and env an agent client would use, run the JSON-RPC handshake, and assert `vulcan_*` tools appear. |
| `log show [--limit <n>] [--session <id>]`                                                                        | Show recent redacted action log records.                                                                                                   |
| `log summary [--limit <n>] [--session <id>]`                                                                     | Summarize recent actions, positions, PnL, errors, and transactions.                                                                        |
| `log report [--limit <n>] [--session <id>]`                                                                      | Build a position and session report from live trader state and local logs.                                                                 |

```bash theme={null}
vulcan agent install --target claude
vulcan agent mcp install --target cursor --scope user --dangerous
```

## `strategy` — strategy runners

Long-running TWAP, grid, and TA strategy runners with ledger-backed pause / resume / finalize lifecycle. See the [Strategies](/cli/strategies) page for the full reference.

| Subcommand                                                                                               | Description                                                                                                                                                                               |
| -------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `twap start ...`                                                                                         | Start a TWAP run.                                                                                                                                                                         |
| `twap resume <run-id> [--from-step <n>]`                                                                 | Resume a paused or incomplete TWAP run.                                                                                                                                                   |
| `grid start ...`                                                                                         | Start a grid trading run.                                                                                                                                                                 |
| `grid resume <run-id> [--from-step <n>]`                                                                 | Resume a paused or incomplete grid run.                                                                                                                                                   |
| `ta start ...`                                                                                           | Start a TA-driven strategy run from a config file or JSON.                                                                                                                                |
| `ta resume <run-id>`                                                                                     | Resume a paused or incomplete TA run.                                                                                                                                                     |
| `runs [--limit <n>]`                                                                                     | List persisted strategy runs.                                                                                                                                                             |
| `status <run-id> [--since-tick <n>] [--include-ledger]`                                                  | Show latest status for a run.                                                                                                                                                             |
| `monitor <run-id> [--include-ledger]`                                                                    | Compact non-blocking monitor state.                                                                                                                                                       |
| `wait-next-tick <run-id> [--after-tick <n>] [--timeout-seconds <s>]`                                     | Wait until a new tick or terminal status.                                                                                                                                                 |
| `report <run-id>`                                                                                        | Show final or latest report.                                                                                                                                                              |
| `reconcile-grid <run-id>`                                                                                | Inspect live grid orders against the persisted ledger.                                                                                                                                    |
| `pause <run-id> [--reason <r>]`                                                                          | Request a running strategy to pause at the next safe point.                                                                                                                               |
| `stop <run-id> [--reason <r>]`                                                                           | Request a strategy to stop permanently at the next safe point.                                                                                                                            |
| `finalize <run-id> [--reason <r>] [--cancel-orders] [--close-position] [--wait] [--timeout-seconds <s>]` | Stop a strategy and optionally clean up live orders or positions.                                                                                                                         |
| `preflight`                                                                                              | Inspect live-readiness for the active wallet without launching anything: wallet identity, password availability, trader registration, collateral, and a remedy command for every blocker. |
| `resume <run-id> [--from-step <n>]`                                                                      | Resume any paused or incomplete strategy run.                                                                                                                                             |

## `ta` — technical analysis

| Subcommand                                                                                              | Description                                                                                                                                     |
| ------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| `compute <symbol> --indicator <name> [--timeframe <tf>] [--period <p>] [--limit <n>] [--params <json>]` | Compute a single indicator over the latest candles. Supported indicators: `sma`, `ema`, `rsi`, `macd`, `bbands`, `atr`, `vwap`, `adx`, `stoch`. |
| `signal <symbol> --spec <json>`                                                                         | Evaluate a trigger spec against the latest indicator value. Example spec: `{"indicator":"rsi","timeframe":"1h","op":"lt","threshold":30}`.      |
| `report <symbol> [--timeframe <tf>]`                                                                    | Multi-indicator snapshot (RSI, MACD, BBands, ATR, ADX).                                                                                         |

```bash theme={null}
vulcan ta compute SOL --indicator rsi --timeframe 1h
vulcan ta compute SOL --indicator macd --params '{"fast":12,"slow":26,"signal":9}'
vulcan ta report SOL --timeframe 4h -o json
```

## Standalone commands

| Command                                     | Description                                                         |
| ------------------------------------------- | ------------------------------------------------------------------- |
| `status`                                    | Check configuration, connectivity, wallet, and registration status. |
| `setup`                                     | Interactive setup wizard for wallet, config, and connectivity.      |
| `version`                                   | Print version and build information.                                |
| `update check [--force]`                    | Check whether a newer Vulcan release is available.                  |
| `agent-context`                             | Print agent runtime context (CONTEXT.md) to stdout.                 |
| `mcp [--allow-dangerous] [--groups <list>]` | Start the local MCP server over stdio.                              |

```bash theme={null}
vulcan status -o json
vulcan update check
vulcan mcp --allow-dangerous --groups market,trade,position,margin
```

<!-- END SOURCE: cli/commands.md -->

---

# Source: `cli/index.md`

**Original URL:** `https://docs.phoenix.trade/cli/index.md`

**Local path:** `cli/index.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# CLI

> Agent and human-friendly CLI for trading Phoenix perpetual futures on Solana.

Vulcan is the official command-line tool for trading [Phoenix](https://phoenix.trade) perpetual futures. It is designed for both human operators and AI agents, with structured JSON output, a local Model Context Protocol (MCP) server, and bundled skills for popular agent clients.

The Vulcan repository is on GitHub: [Ellipsis-Labs/vulcan-cli](https://github.com/Ellipsis-Labs/vulcan-cli).

<Warning>
  Live commands execute irreversible financial transactions on Solana Mainnet. You are responsible for wallet security, agent permissions, and all trading outcomes.
</Warning>

## What you can do

* Manage encrypted wallets, register and configure trader accounts, deposit and withdraw collateral.
* Read live market data: prices, orderbooks, candles, funding rates, recent trades.
* Compute technical indicators (RSI, MACD, Bollinger Bands, ATR, VWAP, ADX, Stoch, SMA, EMA) and evaluate triggers.
* Place, cancel, and modify market and limit orders with optional take-profit and stop-loss.
* Monitor and close positions across cross and isolated margin.
* Run first-class strategy loops for [TWAP, grid, and TA-driven trading](/cli/strategies).
* Trade in local paper mode against live prices with no real funds at risk.
* Install [agent skills](/cli/installation#agent-skills) for Claude Code, Cursor, Codex, and other clients, then expose Vulcan tools through a local MCP server.

## Where to go next

* [Installation](/cli/installation) — install the binary, configure your wallet, and verify connectivity.
* [Command reference](/cli/commands) — every command group with its subcommands and flags.
* [Strategies](/cli/strategies) — detailed reference for the TWAP, grid, and TA strategy runners.

## Output format

Most commands support `-o table` (default) and `-o json`. Use JSON for scripting and agent integrations; structured command responses use a consistent success/error envelope:

```json theme={null}
{ "ok": true, "data": { }, "meta": { } }
```

```json theme={null}
{ "ok": false, "error": { "category": "", "code": "", "message": "", "retryable": false } }
```

<!-- END SOURCE: cli/index.md -->

---

# Source: `cli/installation.md`

**Original URL:** `https://docs.phoenix.trade/cli/installation.md`

**Local path:** `cli/installation.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Installation

> Install the Vulcan CLI and set up your wallet, configuration, and agent integrations.

## Prerequisites

* macOS or Linux.
* `~/.local/bin` on your `PATH` (the default install location).
* A Solana wallet, or you can let Vulcan generate one during setup.

## Install the latest release

```bash theme={null}
curl -fsSL https://github.com/Ellipsis-Labs/vulcan-cli/releases/latest/download/install.sh | sh
```

The installer downloads the release archive, verifies it against `vulcan-checksums-sha256.txt`, and installs the binary to `~/.local/bin/vulcan` by default. Set `VULCAN_INSTALL_DIR` before running to install elsewhere.

## Install a specific version

Replace the tag with any tagged release:

```bash theme={null}
curl -fsSL https://github.com/Ellipsis-Labs/vulcan-cli/releases/download/v0.5.3/install.sh | sh
```

## Build from source

Requires Rust 1.84 or newer. From a clone of [Ellipsis-Labs/vulcan-cli](https://github.com/Ellipsis-Labs/vulcan-cli):

```bash theme={null}
cargo install --path vulcan
```

## Verify

```bash theme={null}
vulcan version
```

## First-run setup

Run the interactive wizard to configure your wallet, RPC and API endpoints, registration, and optional first deposit:

```bash theme={null}
vulcan setup
```

Then confirm everything is wired up:

```bash theme={null}
vulcan status -o json
```

## Configuration file

Vulcan reads `~/.vulcan/config.toml`. A typical configuration looks like:

```toml theme={null}
[network]
rpc_url = "https://api.mainnet-beta.solana.com"
api_url = "https://perp-api.phoenix.trade"
# api_key = "your-api-key"

[wallet]
default = "my-wallet"

[trading]
default_slippage_bps = 50
confirm_trades = true
```

Any of `rpc_url`, `api_url`, or the default wallet can be overridden per-command with the matching global flag (`--rpc-url`, `--api-url`, `-w/--wallet`).

## Environment variables

| Variable                 | Purpose                                                            |
| ------------------------ | ------------------------------------------------------------------ |
| `VULCAN_WALLET_NAME`     | Stored wallet to use (defaults to the configured default wallet).  |
| `VULCAN_WALLET_PASSWORD` | Wallet unlock password. Required for non-interactive MCP sessions. |
| `VULCAN_INSTALL_DIR`     | Custom install directory used by the install script.               |
| `VULCAN_AGENT_TARGET`    | Default agent client target for `vulcan agent install`.            |
| `VULCAN_AGENT_SCOPE`     | Default install scope: `user` or `project`.                        |

## Agent skills

Vulcan ships bundled skill files that teach AI agents how to use it safely. Install them for your client:

```bash theme={null}
vulcan agent install --target claude
vulcan agent install --target cursor
vulcan agent install --target codex
vulcan agent install --target agentskills
```

Supported targets: `claude`, `cursor`, `codex`, `agentskills`. Add `--scope project` for project-local installs where the target supports it.

Inspect what is currently installed:

```bash theme={null}
vulcan agent doctor --target claude
vulcan agent health
```

End-to-end probe — spawn the server with the exact command and env an agent client would use and assert that `vulcan_*` tools come back:

```bash theme={null}
vulcan agent mcp diagnose --target claude --scope user
```

If you migrate to a new Vulcan binary path and want to update `command` / `args` in an existing MCP entry without re-entering your wallet password:

```bash theme={null}
vulcan agent mcp install --target claude --scope user --repair
```

## MCP server

Vulcan can run as a local Model Context Protocol (MCP) server over stdio so agents can call its tools directly. Private keys never leave the local Vulcan process.

Read-only / paper-safe:

```bash theme={null}
vulcan mcp
```

Live-capable:

```bash theme={null}
export VULCAN_WALLET_NAME=my-wallet
export VULCAN_WALLET_PASSWORD=your-password
vulcan mcp --allow-dangerous
```

To have an agent client launch the MCP server automatically, install an MCP config:

```bash theme={null}
vulcan agent mcp install --target cursor --scope user
vulcan agent mcp install --target cursor --scope user --dangerous
```

The `--dangerous` form prompts for wallet name and password, then writes the values into the agent client's MCP config.

To switch the wallet used by an already-installed MCP server:

```bash theme={null}
vulcan agent mcp set-wallet <wallet-name> --target claude --scope user
```

Continue to the [command reference](/cli/commands) or jump straight to [strategies](/cli/strategies).

<!-- END SOURCE: cli/installation.md -->

---

# Source: `cli/strategies.md`

**Original URL:** `https://docs.phoenix.trade/cli/strategies.md`

**Local path:** `cli/strategies.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Strategies

> First-class strategy runners: TWAP, grid, and TA-driven trading on Phoenix perpetuals.

Vulcan's `strategy` command group hosts long-running execution loops that are too stateful for one-shot orders. Each run is ledger-backed, supports `pause` / `resume` / `finalize` lifecycle commands, and can be started in detached mode so the runner keeps ticking in the background. Tick logs and ledgers are persisted under `~/.vulcan/strategy-runs`.

Three runners ship today:

* **TWAP** — split a target size across timed slices.
* **Grid** — maintain layered limit orders across a price band.
* **TA** — rule-based runner over technical indicators for entry and exit.

## Execution modes

Every strategy accepts `--mode`. Pick the one that matches your tolerance for live risk while testing:

| Mode           | Behavior                                                             |
| -------------- | -------------------------------------------------------------------- |
| `paper`        | Simulated against live prices. No wallet or chain activity. Default. |
| `dry-run`      | Builds and logs each step but does not submit transactions.          |
| `confirm-each` | Submits live orders, prompting for confirmation before each step.    |
| `auto-execute` | Submits live orders without prompting. Use with guardrails.          |

## Margin mode

For TWAP and grid runs, `--margin-mode` selects how collateral is held:

| Mode       | Behavior                                                                                 |
| ---------- | ---------------------------------------------------------------------------------------- |
| `cross`    | Uses the trader's main cross-margin account. Default.                                    |
| `isolated` | Opens an isolated subaccount. Pair with `--isolated-collateral` for the opening deposit. |

## Guardrails

All three runners accept the same safety flags. Any breach pauses the run and records the reason in the ledger:

| Flag                                 | Description                                                   |
| ------------------------------------ | ------------------------------------------------------------- |
| `--max-total-notional-usdc <amount>` | Cap on total live notional placed by the run.                 |
| `--max-step-notional-usdc <amount>`  | Cap on a single tick's live notional.                         |
| `--max-price-drift-bps <bps>`        | Pause if mark price drifts this far from the run-start price. |
| `--max-exposure-ratio <ratio>`       | Pause if position notional / equity exceeds this ratio.       |
| `--reconcile-attempts <n>`           | Number of history reconciliation attempts per live step.      |
| `--reconcile-delay-ms <ms>`          | Milliseconds between reconciliation attempts.                 |

## TWAP

TWAP slices a target size across N equally spaced intervals so each fill nudges the average price toward the time-weighted mean.

### `vulcan strategy twap start`

| Flag                             | Description                                                                      |
| -------------------------------- | -------------------------------------------------------------------------------- |
| `--symbol <s>`                   | Market symbol (e.g. `SOL`).                                                      |
| `--side <buy\|sell>`             | Trade direction.                                                                 |
| `--notional-usdc <amount>`       | Total TWAP notional in USDC. Mutually exclusive with `--tokens`.                 |
| `--tokens <amount>`              | Total TWAP size in base-asset tokens. Mutually exclusive with `--notional-usdc`. |
| `--slices <n>`                   | Number of slices.                                                                |
| `--interval-seconds <s>`         | Seconds between slices. Default `60`.                                            |
| `--mode <m>`                     | Execution mode. Default `paper`.                                                 |
| `--margin-mode <m>`              | Margin mode. Default `cross`.                                                    |
| `--isolated-collateral <amount>` | USDC to transfer when opening the first isolated live order.                     |
| `--run-label <label>`            | Optional human-readable label stored with the run.                               |
| `--detached`                     | Start the runner in the background and return immediately with the run ID.       |

Plus the [guardrail flags](#guardrails) above.

### `vulcan strategy twap resume`

```
vulcan strategy twap resume <run-id> [--from-step <n>]
```

Resume a paused or incomplete TWAP run, optionally starting at a specific step.

### Example

```bash theme={null}
vulcan strategy twap start \
  --symbol SOL \
  --side buy \
  --notional-usdc 5000 \
  --slices 10 \
  --interval-seconds 300 \
  --mode auto-execute \
  --max-step-notional-usdc 600 \
  --max-price-drift-bps 75 \
  --detached
```

## Grid

Grid trading lays buy levels below the mark and sell levels above the mark, then maintains the ladder as fills happen. The runner can either be bounded by `--ticks` or run indefinitely with `--run-until-stopped`.

### `vulcan strategy grid start`

| Flag                                    | Description                                                                                     |
| --------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `--symbol <s>`                          | Market symbol (e.g. `SOL`).                                                                     |
| `--lower-price <p>`                     | Lower grid boundary price. Required unless `--center-on-mark` is set.                           |
| `--upper-price <p>`                     | Upper grid boundary price. Required unless `--center-on-mark` is set.                           |
| `--center-on-mark`                      | Center the grid on the current mark price at launch. Requires `--width-pct`.                    |
| `--width-pct <pct>`                     | Half-width of the grid as a percentage of mark (e.g. `1.0` = ±1%). Requires `--center-on-mark`. |
| `--levels-per-side <n>`                 | Number of buy levels below mark and sell levels above mark.                                     |
| `--tokens-per-level <t>`                | Order size per level in base-asset tokens. Mutually exclusive with `--size-lots-per-level`.     |
| `--size-lots-per-level <n>`             | Order size per level in base lots. Mutually exclusive with `--tokens-per-level`.                |
| `--bid-level PRICE:SIZE_LOTS[:TP][:SL]` | Fully custom bid level. Repeatable, replaces generated bids.                                    |
| `--ask-level PRICE:SIZE_LOTS[:TP][:SL]` | Fully custom ask level. Repeatable, replaces generated asks.                                    |
| `--take-profit-spacing <p>`             | Distance from entry to take-profit for generated levels.                                        |
| `--stop-loss-spacing <p>`               | Distance from entry to stop-loss for generated levels.                                          |
| `--interval-seconds <s>`                | Seconds between maintenance ticks. Default `60`.                                                |
| `--ticks <n>`                           | Maximum ticks to run, including the initial placement tick. Default `60`.                       |
| `--run-until-stopped`                   | Keep running maintenance ticks until paused or stopped.                                         |
| `--stale-after-seconds <s>`             | Seconds without a tick before status reports this run as stale.                                 |
| `--mode <m>`                            | Execution mode. Default `paper`.                                                                |
| `--margin-mode <m>`                     | Margin mode. Default `cross`.                                                                   |
| `--isolated-collateral <amount>`        | USDC to transfer when opening the first isolated live order.                                    |
| `--run-label <label>`                   | Optional human-readable label.                                                                  |
| `--slide`                               | Allow live multi-limit orders to slide to the top of book if crossing.                          |
| `--detached`                            | Start in the background and return immediately with the run ID.                                 |

Plus the [guardrail flags](#guardrails) above.

### `vulcan strategy grid resume`

```
vulcan strategy grid resume <run-id> [--from-step <n>]
```

Resume a paused or incomplete grid run.

### Example

```bash theme={null}
vulcan strategy grid start \
  --symbol SOL \
  --center-on-mark \
  --width-pct 2.5 \
  --levels-per-side 5 \
  --tokens-per-level 0.5 \
  --run-until-stopped \
  --mode auto-execute \
  --max-total-notional-usdc 10000 \
  --detached
```

## TA

The TA runner evaluates rules of the form `(condition, action)` over indicator values on a chosen timeframe. Conditions reference indicators like `rsi`, `macd`, `ema`, etc.; actions place or close positions when triggered. Configs are JSON, supplied inline or via a file. TA margin settings are part of that JSON config (`margin_mode` and `isolated_collateral`), not separate CLI flags.

### `vulcan strategy ta start`

| Flag                   | Description                                                                    |
| ---------------------- | ------------------------------------------------------------------------------ |
| `--config-file <path>` | Path to a JSON config file. Mutually exclusive with `--config-json`.           |
| `--config-json <json>` | Inline JSON config. Mutually exclusive with `--config-file`.                   |
| `--mode <m>`           | Execution mode. Default `paper`.                                               |
| `--max-ticks <n>`      | Maximum ticks to run. Ignored when `--run-until-stopped` is set. Default `60`. |
| `--run-until-stopped`  | Keep running until paused or stopped.                                          |
| `--run-label <label>`  | Optional human-readable label.                                                 |
| `--detached`           | Start in the background and return immediately with the run ID.                |

Plus the [guardrail flags](#guardrails) above.

### `vulcan strategy ta resume`

```
vulcan strategy ta resume <run-id>
```

Resume a paused or incomplete TA strategy run.

### Example

```bash theme={null}
vulcan strategy ta start \
  --config-file ./ema-cross-sol.json \
  --mode paper \
  --run-until-stopped \
  --detached
```

For ad-hoc indicator queries outside the strategy framework, use the top-level [`vulcan ta`](/cli/commands#ta-technical-analysis) commands.

## Lifecycle commands

These apply to any strategy run, regardless of type. Most take the `<run-id>` returned by `start` (`runs` and `preflight` do not). List recent run IDs with `vulcan strategy runs`.

| Command                                                                                                  | Description                                                                                                                                     |
| -------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| `runs [--limit <n>]`                                                                                     | List persisted strategy runs. Default limit 20.                                                                                                 |
| `status <run-id> [--since-tick <n>] [--include-ledger]`                                                  | Show latest status. `--since-tick` returns only ticks newer than the given index.                                                               |
| `monitor <run-id> [--include-ledger]`                                                                    | Compact non-blocking monitor state.                                                                                                             |
| `wait-next-tick <run-id> [--after-tick <n>] [--timeout-seconds <s>] [--include-ledger]`                  | Block until a new tick or terminal status is observed. Default timeout 90 seconds.                                                              |
| `report <run-id>`                                                                                        | Show the final (or latest) report.                                                                                                              |
| `reconcile-grid <run-id>`                                                                                | Inspect live grid orders against the persisted ledger without mutating state.                                                                   |
| `pause <run-id> [--reason <r>]`                                                                          | Request the runner to pause at the next safe point.                                                                                             |
| `stop <run-id> [--reason <r>]`                                                                           | Request the runner to stop permanently at the next safe point.                                                                                  |
| `finalize <run-id> [--reason <r>] [--cancel-orders] [--close-position] [--wait] [--timeout-seconds <s>]` | Stop the runner and optionally cancel open orders and close the position on the strategy symbol.                                                |
| `preflight`                                                                                              | Pre-launch readiness check: wallet identity, password availability, trader registration, collateral. Lists every blocker with a remedy command. |
| `resume <run-id> [--from-step <n>]`                                                                      | Resume any paused or incomplete strategy run.                                                                                                   |

### Typical operating loop

```bash theme={null}
RUN_ID=$(vulcan strategy grid start ... --detached -o json | jq -r '.data.run_id')

vulcan strategy monitor "$RUN_ID" -o json
vulcan strategy wait-next-tick "$RUN_ID" --timeout-seconds 120

vulcan strategy pause "$RUN_ID" --reason "checking risk"
vulcan strategy resume "$RUN_ID"

vulcan strategy finalize "$RUN_ID" --cancel-orders --close-position --wait
```

<!-- END SOURCE: cli/strategies.md -->

---

# Source: `incentive-programs/flight-club.md`

**Original URL:** `https://docs.phoenix.trade/incentive-programs/flight-club.md`

**Local path:** `incentive-programs/flight-club.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Flight Club

> Earn by trading on Phoenix

Flight Club is a four-week rewards program that distributes 420,000 USDC to traders using Phoenix.

## Program schedule

Flight Club runs from July 27 through August 23.

* 420,000 USDC is distributed over 28 days
* 15,000 USDC is distributed each day
* Rewards are calculated and distributed daily, within 24 hours of the day's end.

## How you earn

Your daily rewards are based on activity across Phoenix:

* **Trading:** Every trade contributes to your share of the day's rewards.
* **Holding positions:** Keeping positions open also counts, so both size and conviction are rewarded.
* **Referring traders:** Volume from traders you refer contributes to your rewards. Referred traders receive a fee discount, and Flight Club rewards are additional to the existing [Referral Program](/incentive-programs/referral-program).
* **Using Phoenix consistently:** Trade consistently to increase your rewards.

## Claiming rewards

Rewards are paid daily in USDC to your Phoenix account. Claim them from the rewards page in the Phoenix app.

Rewards have no vesting period, lockup, or cliff. They are claimable once distributed, within 24 hours of the day's end.

<!-- END SOURCE: incentive-programs/flight-club.md -->

---

# Source: `incentive-programs/referral-program.md`

**Original URL:** `https://docs.phoenix.trade/incentive-programs/referral-program.md`

**Local path:** `incentive-programs/referral-program.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Referral Program

> Earn by referring traders to Phoenix

## Referral Program

Phoenix's referral program lets traders earn a share of fees generated by users they refer, across two tiers.

## Refer and earn

Traders with at least \$10,000 in lifetime volume can generate a referral code in the app.

Each user gets one code with a customizable name. Codes have a limited number of uses at launch, with limits expanding over time.

* referrers earn 20% of trading fees from direct referrals
* referrers earn 10% of trading fees from second-tier referrals
* rewards accrue weekly in USDC
* the fee-share applies to the first \$100M in volume per referred user

## Using a referral code

New users enter a referral code during sign-up at [phoenix.trade](http://phoenix.trade).

Referral codes apply a 10% discount on trading fees across perpetual markets for the duration of the \$100M volume cap.

Developers and SDK integrations can activate a referral code with `/v1/referral/activate-tx`. This route always requires a referral code, and the trader authority must sign the transaction. A separate payer can fund transaction fees and account rent. See [Trader Onboarding](/sdk/register#with-a-referral-code).

## Claiming rewards

* the referral page becomes visible after trading \$10,000 of volume
* rewards update weekly
* there is a \$1 minimum claim
* claimed rewards are deposited in USDC to the referrer's spot balance
* unclaimed rewards do not expire
* referral relationships are permanent and persist for the duration of the volume cap

## Program rules

* self-referrals are blocked
* abusive behavior such as wash trading can cause revocation
* existing referral relationships remain intact if a code is later revoked

## Support

For questions about the referral program, reach out to the team on [Discord](https://discord.gg/phoenixtrade).

## See also

* [Fees](/phoenix/matching-engine/fees)
* [Builder Codes](/builder-codes)
* [Support](/phoenix/faq/support)

<!-- END SOURCE: incentive-programs/referral-program.md -->

---

# Source: `index.md`

**Original URL:** `https://docs.phoenix.trade/index.md`

**Local path:** `index.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Getting Started

> Getting started with Phoenix

<div style={{ textAlign:"center" }}>
  <img src="https://mintcdn.com/ellipsislabs/ATrCr5hWKehgh2fc/logo/Logo_Orange.svg%20(1).svg?fit=max&auto=format&n=ATrCr5hWKehgh2fc&q=85&s=6c89479be686584b21e77450984f4c26" alt="Phoenix" style={{ height:"80px" }} width="377" height="61" data-path="logo/Logo_Orange.svg (1).svg" />
</div>

## What is Phoenix?

Phoenix is a non-custodial, decentralized exchange for perpetual futures built on the Solana blockchain.

<CardGroup cols={2}>
  <Card title="Non-Custodial" icon="shield-check" color="#FFA06E">
    Maintain full control of your assets at all times
  </Card>

  <Card title="Decentralized" icon="network-wired" color="#FFA06E">
    Built on Solana for fast, transparent transactions
  </Card>

  <Card title="Perpetual Futures" icon="chart-line" color="#FFA06E">
    Trade perpetual contracts with leverage
  </Card>

  <Card title="No Expiration" icon="infinity" color="#FFA06E">
    Contracts have no date expiration
  </Card>
</CardGroup>

***

## Accessing Phoenix

Phoenix is available at [phoenix.trade](http://phoenix.trade). Phoenix is not available in the U.S. or sanctioned jurisdictions.

## Connect Wallet and Fund Account

Before you begin, make sure you can buy and withdraw USDC from a cryptocurrency exchange. Phoenix currently accepts USDC as collateral.

<Steps>
  <Step title="Choose How to Sign In">
    <CardGroup cols={2}>
      <Card title="Option A: Email or Google" icon="user" color="#FFA06E">
        **Recommended for first-time users**

        Select **Start Trading**, then log in with Google or enter your email. Phoenix creates a wallet for you through Privy, so you do not need to install a wallet extension.
      </Card>

      <Card title="Option B: Browser Wallet" icon="wallet" color="#FFA06E">
        **Recommended browser wallets**

        Install [Solflare](https://www.solflare.com/download/), [Phantom](https://phantom.com/download), or [Backpack](https://backpack.app/download), then create a wallet. If this is your first browser wallet, follow [Solflare's step-by-step setup guide](https://www.solflare.com/guides/how-to-set-up-your-first-crypto-wallet-with-solflare-step-by-step/).

        Then select **Start Trading**, select **Connect new wallet**, and choose your wallet.
      </Card>
    </CardGroup>
  </Step>

  <Step title="Fund Your Account with USDC">
    Buy USDC from a cryptocurrency exchange, withdraw it to the wallet you use with Phoenix, then deposit it from that wallet into Phoenix:

    1. Buy USDC on an exchange.
    2. Copy your wallet address from the Phoenix account menu. On the exchange, withdraw your USDC to that address and select **Solana** as the withdrawal network.
    3. After the USDC arrives in your wallet, return to Phoenix and select **Deposit**.
    4. Enter the amount, select **Submit Deposit**, and follow the on-screen prompts to confirm the deposit.
  </Step>

  <Step title="Place a Basic Trade">
    After funding your account:

    1. Select the market you want to trade.
    2. Choose **Market** to trade at available prices, or **Limit** to set a price. A limit order may remain open if it cannot fill at your price or better.
    3. Choose **Long/Buy** or **Short/Sell**, then enter the size. For a limit order, also enter the **Limit Price**.
    4. Review the order details, select **Place Order**, and confirm the order.

    Learn more: [Order Types](phoenix/matching-engine/order-types), [Fees](phoenix/matching-engine/fees), and [Risk Warning](phoenix/margin-and-risk/risk-warning).
  </Step>
</Steps>

<!-- END SOURCE: index.md -->

---

# Source: `phoenix/collateral-and-accounts/accounts.md`

**Original URL:** `https://docs.phoenix.trade/phoenix/collateral-and-accounts/accounts.md`

**Local path:** `phoenix/collateral-and-accounts/accounts.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Accounts

> How Phoenix accounts work and how cross and isolated risk differ

Phoenix separates a wallet authority from its trader accounts. The wallet signs transactions. Trader accounts hold collateral, positions, and resting orders.

Each trader account is a program-derived address (PDA) derived from:

* wallet authority
* `portfolio_index`
* `subaccount_index`

The pair `(portfolio_index, subaccount_index)` defines which account is being addressed for that authority.

## Authority and position authority

Each trader account has an `authority` and a `position_authority`.

The `authority` is the wallet authority used to derive and own the trader account. It has full account control: it can deposit and withdraw collateral, place and cancel orders, manage stop losses and conditional orders, create and sync subaccounts, and update the account's `position_authority`.

The `position_authority` is a trading signer stored on the trader account. New trader accounts start with `position_authority` set to the same wallet as `authority`, but the authority can delegate it to another wallet, such as an embedded app wallet. A position authority can sign trading actions like placing orders, cancelling orders, and managing stop losses or other conditional orders.

A `position_authority` is not a collateral owner. It cannot withdraw collateral from Phoenix and cannot call authority-only account management instructions such as `WithdrawFunds`, `DepositFunds`, or `DelegateTrader`. Some collateral transfers between trader accounts under the same wallet authority may allow a position authority signer, but a position authority cannot move collateral out to the wallet.

To set or rotate this signer from an integration, see [Set position authority](/sdk/accounts#set-position-authority) in the SDK docs.

## Portfolio index

`portfolio_index` partitions a wallet's Phoenix state into independent portfolios.

Each `portfolio_index` has its own collateral, positions, orders, and risk. Positions and collateral in one portfolio do not margin positions in another portfolio.

This lets a single wallet authority maintain separate trading books without sharing collateral or liquidation risk between them.

## Subaccount index

Within a portfolio, `subaccount_index` selects either the cross account or an isolated account.

### Cross account

`subaccount_index = 0` is the cross account for the portfolio.

The cross account uses one shared collateral pool across its active positions. PnL, funding, margin requirements, and order margin all affect the same account health.

Capacity limits:

* up to `128` active positions
* up to `64` resting bid limit orders per market
* up to `64` resting ask limit orders per market

### Isolated accounts

`subaccount_index > 0` is an isolated account under the same portfolio.

An isolated account is used for a single isolated position. Collateral is moved from the portfolio's cross account into the isolated account when the isolated position is opened or funded.

The isolated account has its own collateral and liquidation boundary. Losses in the isolated account do not automatically draw from the cross account after collateral has been allocated.

When the isolated position is closed, a crank can sweep remaining collateral back from the isolated account to the portfolio's cross account.

## Account model

For a given wallet authority:

* `(portfolio_index = 0, subaccount_index = 0)` is the first portfolio's cross account
* `(portfolio_index = 0, subaccount_index > 0)` is an isolated account under the first portfolio
* `(portfolio_index = 1, subaccount_index = 0)` is a separate cross account with independent collateral and positions
* `(portfolio_index = 1, subaccount_index > 0)` is an isolated account under that separate portfolio

The important boundary is the `portfolio_index`. Subaccounts inside one portfolio relate to that portfolio's cross account. Different portfolios do not share margin.

<!-- END SOURCE: phoenix/collateral-and-accounts/accounts.md -->

---

# Source: `phoenix/collateral-and-accounts/collateral.md`

**Original URL:** `https://docs.phoenix.trade/phoenix/collateral-and-accounts/collateral.md`

**Local path:** `phoenix/collateral-and-accounts/collateral.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Collateral

> How Phoenix uses USDC collateral

Phoenix perps are currently margined in USDC. Deposits, withdrawals, margin checks, PnL, and funding all resolve against the account's USDC collateral balance.

## Collateral asset

Phoenix currently supports USDC as its only collateral asset. Other collateral assets may be added in the future.

Solana USDC is deposited through the Ember contract first before it reaches the Phoenix exchange. Ember wraps Solana USDC `1:1` into Phoenix's exchange-side collateral representation. Withdrawals perform the reverse conversion.

## Ember

Ember is the proxy contract between standard Solana USDC and Phoenix collateral.

Deposits:

1. USDC moves from your wallet token account into Ember.
2. Ember wraps the same atomic amount `1:1`.
3. Phoenix credits the collateral to your trader account.

Withdrawals:

1. Phoenix withdraws collateral from your trader account.
2. Ember unwraps the same atomic amount `1:1`.
3. USDC returns to your wallet.

Ember does not set exchange rates, apply haircuts, or introduce another collateral asset. The amount transferred into Ember is the amount credited through the wrapper. The amount burned on withdrawal is the amount released back as Solana USDC.

## Account collateral and effective collateral

`deposited_collateral` is the USDC collateral balance credited to a Phoenix trader account.

`effective_collateral` is the collateral value Phoenix uses for risk checks. It includes deposited collateral, unsettled funding, and unrealized PnL:

$$
C_{\text{effective}} = C_{\text{deposited}} + U + F_{\text{unsettled}}
$$

Effective collateral is not a separate balance or token. It is the margin value used for health checks, liquidations, and withdrawal eligibility.

## Withdrawals

Phoenix rate-limits protocol-wide collateral outflows with a global withdraw queue. The queue applies to exchange-wide withdrawals, not just activity on one account.

* A withdrawal clears immediately if the queue is empty and the request fits within the current withdrawal budget.
* Otherwise, the withdrawal is queued.
* Queued withdrawals are processed in FIFO order as budget replenishes over time.
* Queued withdrawals are not partially filled.
* If the account no longer has enough withdrawable collateral when the request reaches the front of the queue, the request can be dropped instead of processed.

## Reference

### Contract and mint addresses

* Ember program id: `EMBERpYNE6ehWmXymZZS2skiFmCa9V5dp14e1iduM5qy`
* Wallet USDC mint: `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`
* Phoenix USDC mint: `PhUsd11YkbjSaWjFncfAAmatntsjx3MgDR9B6g1ks3A`

### Current withdraw parameters

Snapshot verified from live mainnet exchange accounts on `2026-04-20`:

* `max_budget`: `2,000,000` Phoenix USDC
* `replenish_amount_per_slot`: `450` Phoenix USDC per slot
* `withdrawal_fee`: `0`
* `enqueueing_fee`: `0`
* `deposit_cooldown_period_in_slots`: `0`

<!-- END SOURCE: phoenix/collateral-and-accounts/collateral.md -->

---

# Source: `phoenix/collateral-and-accounts/cross-chain-deposits.md`

**Original URL:** `https://docs.phoenix.trade/phoenix/collateral-and-accounts/cross-chain-deposits.md`

**Local path:** `phoenix/collateral-and-accounts/cross-chain-deposits.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Cross-chain Deposits

> How to deposit source tokens to Phoenix from other chains

Phoenix collateral lives on Solana as USDC. If your source token is on another chain, Phoenix routes the deposit through [Relay](https://relay.link), which swaps and bridges the funds to Solana USDC before crediting your trader account.

## Supported source chains

Cross-chain deposits are currently supported from:

* Ethereum
* Base
* Arbitrum
* HyperEVM

## How it works

1. You select a supported source chain and source token in the deposit UI.
2. The UI displays a sample quote from Relay, including the amount of Solana USDC you will receive after fees.
3. After you confirm, Relay executes the swap and bridges the funds to Solana.
4. The resulting Solana USDC is deposited through Ember and credited to your Phoenix trader account.

## Quotes

The sample quote shown in the UI is provided by Relay and varies based on the cost of gas on the source chain at the time of the request. The final amount credited may differ slightly from the sample quote if gas prices move between when the quote is displayed and when the transaction is executed.

## Troubleshooting

If a cross-chain deposit does not arrive or you run into any issues, file a ticket in the [Phoenix Discord](https://discord.gg/phoenixtrade) and include your source-chain transaction signature. The team can use the signature to trace the deposit with Relay and resolve the issue.

<!-- END SOURCE: phoenix/collateral-and-accounts/cross-chain-deposits.md -->

---

# Source: `phoenix/faq/faq.md`

**Original URL:** `https://docs.phoenix.trade/phoenix/faq/faq.md`

**Local path:** `phoenix/faq/faq.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# FAQ

> Answers to the most common execution, margin, and account questions

## Why can effective collateral differ from my deposited collateral?

Effective collateral is the value Phoenix uses for risk checks. It is not a separate balance or token.

At a high level:

$$
C_{\text{effective}} = C_{\text{deposited}} + U^{+}_{\text{discounted}} + U^{-} + F_{\text{unsettled}}
$$

Deposited collateral is the USDC collateral balance on the trader account. Positive unrealized PnL may be discounted before it counts toward margin, negative unrealized PnL counts in full, and unsettled funding can move effective collateral up or down before it is reflected as settled collateral.

See [Collateral](/phoenix/collateral-and-accounts/collateral) for the account-level collateral model and [Margin Math](/phoenix/margin-and-risk/margin-math) for the risk calculation.

## Do subaccounts share risk?

It depends on the account type.

Within a portfolio, `subaccount_index = 0` is the cross account. Positions, collateral, funding, PnL, and resting-order margin all feed into one shared account health calculation.

`subaccount_index > 0` is used for isolated accounts. Collateral is moved from the portfolio's cross account into the isolated account, and that isolated account has its own liquidation boundary. Losses in the isolated account do not automatically pull more collateral from the cross account after collateral has been allocated. Unused or remaining collateral in the isolated account may be swept back to the cross account.

Different `portfolio_index` values are separate portfolios. They do not share collateral, positions, or liquidation risk.

See [Accounts](/phoenix/collateral-and-accounts/accounts) for the full account model.

## Are builder fees the same as trading fees?

No. Flight builder fees are separate from Phoenix maker/taker fees and are additive when an order is routed through a builder.

See [Fees](/phoenix/matching-engine/fees) and [Builder Codes](/builder-codes).

## Why does positive PnL not fully count toward margin?

Positive unrealized PnL is not the same as deposited collateral. It depends on the current mark price, and that mark can move.

Phoenix can discount positive unrealized PnL before counting it toward effective collateral. Negative unrealized PnL counts in full.

The discount reduces the risk that unstable or manipulated mark prices create too much usable collateral. This matters more for volatile or lower-liquidity assets, where mark prices can be easier to move or temporarily distorted, so the market-specific discount can differ by asset. Market-specific `uPnL risk factor` values are shown in [Market Parameters](/phoenix/market-parameters).

See [Margin Math](/phoenix/margin-and-risk/margin-math) for the effective collateral calculation.

## Why did my liquidation price move when I did not touch that market?

Liquidation price is an estimate, not a fixed promise.

Phoenix estimates liquidation price from your collateral, position size, entry price, mark price, leverage tier, funding, and the rest of the account. In cross margin, other positions can change effective collateral and maintenance requirements, so the estimated liquidation price for one market can move even if you did not trade that market.

Common reasons it moves:

* Mark price changes
* Funding accrues or settles
* Another cross-margin position gains or loses value
* Resting risk-increasing orders change margin requirements
* The market's leverage tier or risk parameters change

See [Margin Math](/phoenix/margin-and-risk/margin-math), [Mark Price](/phoenix/margin-and-risk/mark-price), and [Funding](/phoenix/margin-and-risk/funding-rate).

## Why was my resting order cancelled before I was liquidated?

Phoenix can cancel risk-increasing resting limit orders before liquidating filled positions.

This can happen when effective collateral falls below cancel margin. A resting order is risk-increasing if it could add exposure or increase required margin when filled. Cancelling it can improve account health without touching open positions.

See [Risk Tiers And Liquidation](/phoenix/margin-and-risk/liquidations), [Account Health](/phoenix/margin-and-risk/account-health), and [Leverage Tiers](/phoenix/margin-and-risk/leverage-tiers).

## What is the difference between liquidation and ADL?

* liquidation tries to close the risky account through normal market or backstop paths
* ADL reduces profitable counterparties when the exchange needs a stronger last-resort protection

See [Risk Tiers And Liquidation](/phoenix/margin-and-risk/liquidations).

<!-- END SOURCE: phoenix/faq/faq.md -->

---

# Source: `phoenix/faq/support.md`

**Original URL:** `https://docs.phoenix.trade/phoenix/faq/support.md`

**Local path:** `phoenix/faq/support.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Support

> Get help and stay updated

## General Support

For trading FAQs, support, or to open a help ticket, join the Phoenix Discord: [discord.gg/phoenixtrade](https://discord.gg/phoenixtrade)

Follow Phoenix on X for updates and announcements: [x.com/phoenixtrade](https://x.com/phoenixtrade)

## Connect Discord

The best way to get support and share feedback is through Discord. After signing up for Phoenix, connect your Discord account through the menu in the top right to unlock the feedback channels.

<img src="https://mintcdn.com/ellipsislabs/ATrCr5hWKehgh2fc/images/image(12).png?fit=max&auto=format&n=ATrCr5hWKehgh2fc&q=85&s=1506c3ce8c012e8658d9c9de116e79d7" alt="Image(12)" width="2316" height="1296" data-path="images/image(12).png" />

<!-- END SOURCE: phoenix/faq/support.md -->

---

# Source: `phoenix/glossary.md`

**Original URL:** `https://docs.phoenix.trade/phoenix/glossary.md`

**Local path:** `phoenix/glossary.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Glossary

> Key Phoenix trading, execution, and risk terms

**Account health:** The overall risk condition of a trader account, computed from effective collateral versus margin thresholds.

**ADL:** Auto-deleveraging. A last-resort mechanism that reduces profitable counterparties when liquidation and backstop paths are not enough.

**Backstop liquidation:** A more severe liquidation path used after the ordinary maintenance/liquidation threshold has already been breached.

**Builder fee:** The additional fee charged when an order is routed through Flight.

**Cancel margin:** The threshold below which Phoenix can force-cancel risk-increasing resting orders.

**Discounted unrealized PnL:** Positive unrealized PnL after applying the market's uPnL risk factor. Negative unrealized PnL is counted in full.

**Effective collateral:** Deposited collateral plus discounted unrealized PnL and unsettled funding:

$$
C_{\text{effective}} = C_{\text{deposited}} + U_{\text{discounted}} + F_{\text{unsettled}}
$$

**Entry price:** The effective average basis of the active position. Phoenix derives it from virtual quote position divided by active base position.

**FIFO order book:** The normal resting order book where better price fills first and equal-price orders fill in time order.

**Funding:** Periodic cash flow between longs and shorts based on mark-index basis.

**High-risk margin:** The deepest standard threshold before ADL-style handling becomes relevant.

**Index price:** The external spot reference used as the anchor for funding and mark-price construction.

**Initial margin:** The collateral requirement to open or safely maintain current exposure, based on leverage tiers and order margin.

**Isolated margin:** A margin mode where a child account has its own dedicated collateral pool.

**Liquidation:** Forced risk reduction once effective collateral falls below maintenance requirements.

**Maintenance margin:** The minimum effective collateral needed before normal liquidation begins.

**Mark price:** The reference price Phoenix uses for PnL, liquidations, funding, and trigger orders.

**Open interest:** The total outstanding long and short exposure in a market.

**Pending funding / unsettled funding:** Funding that has accrued but has not yet been formally settled into collateral.

**PnL:** Profit and loss.

**Risk score:** A numerical health metric used across Phoenix state tooling. Lower is healthier; around `1000` is near liquidation.

**Self-trade prevention:** Matching-engine behavior that prevents a trader from filling against their own resting liquidity.

**Spline liquidity:** Phoenix's region-based liquidity model around a spline mid price.

**Subaccount:** A child account under the same wallet authority. In Phoenix, subaccounts above `0` are used for isolated risk.

**Unrealized PnL:** Profit or loss on an open position at the current mark price.

<!-- END SOURCE: phoenix/glossary.md -->

---

# Source: `phoenix/margin-and-risk/account-health.md`

**Original URL:** `https://docs.phoenix.trade/phoenix/margin-and-risk/account-health.md`

**Local path:** `phoenix/margin-and-risk/account-health.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Account Health

> How Phoenix turns collateral and margin into health states and risk tiers

Phoenix expresses risk both as named tiers and as a numerical risk score.

## Risk tiers

| Tier                   | Meaning                                               |
| ---------------------- | ----------------------------------------------------- |
| `Safe`                 | Effective collateral is above initial margin          |
| `AtRisk`               | Below initial margin but not yet cancellable          |
| `Cancellable`          | Risk-increasing resting orders can be force-cancelled |
| `Liquidatable`         | Market-order liquidation can begin                    |
| `BackstopLiquidatable` | The account is beyond normal liquidation comfort      |
| `HighRisk`             | Deeply stressed and potentially ADL-eligible          |

## Risk score

A common Phoenix state score is:

* if maintenance margin is $0$, the score is $0$
* if effective collateral is positive, the score is approximately $\left(\frac{\text{maintenance margin}}{\text{effective collateral}}\right) \times 1000$
* if effective collateral is zero or negative, the score rises above $1000$ with an underwater penalty

Interpretation:

* lower is healthier
* around $1000$ means the account is at or through the liquidation boundary
* above $1000$ means the account is underwater

## What improves health

* adding collateral
* reducing positions
* cancelling risk-increasing resting orders
* receiving positive funding
* positive mark-price movement on your exposure

## What worsens health

* adverse price moves
* paying funding
* adding more size
* placing more risk-increasing orders
* moving collateral out of the account

## Why the same position can have a different health state tomorrow

Because health is portfolio-aware and dynamic:

* mark price changes
* funding changes
* other positions in cross margin change
* positive-uPnL credit can be discounted

<!-- END SOURCE: phoenix/margin-and-risk/account-health.md -->

---

# Source: `phoenix/margin-and-risk/entry-price-pnl.md`

**Original URL:** `https://docs.phoenix.trade/phoenix/margin-and-risk/entry-price-pnl.md`

**Local path:** `phoenix/margin-and-risk/entry-price-pnl.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Entry Price And PnL

> How Phoenix tracks entry price, unrealized PnL, and realized PnL

Phoenix derives entry price from the open position's base size and virtual quote position.

For an open position:

$$
\text{entry price}
= \frac{\left|\text{virtual quote lot position}\right|}
{\left|\text{base lot position}\right|}
$$

If `base_lot_position` is zero, the position has no active entry price.

## When entry price changes

Entry price changes when a fill increases exposure.

* if you are flat or long and buy more, Phoenix recalculates the long entry price
* if you are flat or short and sell more, Phoenix recalculates the short entry price
* maker, taker, and spline fills follow the same accounting rules

Reducing a position does not change the remaining entry price. It realizes PnL on the closed portion and preserves the average basis for the open remainder.

A full close removes the entry price. A flip through zero creates a new entry price for the residual position.

## PnL

Unrealized PnL is based on mark price versus entry price:

$$
\text{unrealized PnL}
= \text{position size} \times \left(\text{mark price} - \text{entry price}\right)
$$

* long positions gain when mark price rises
* short positions gain when mark price falls

Realized PnL is created when a fill reduces, closes, or flips an existing position.

## What does not change entry price

The following do not change entry price by themselves:

* placing a resting limit order
* funding settlement
* trading fees
* builder fees

Funding and fees affect collateral separately. They are not included in `virtual_quote_lot_position`.

If an opposite-side limit order rests, entry price is unchanged. If it executes, the filled portion follows the normal fill rules above.

## See also

* [Mark Price](/phoenix/margin-and-risk/mark-price)
* [Funding](/phoenix/margin-and-risk/funding-rate)
* [Fees](/phoenix/matching-engine/fees)

<!-- END SOURCE: phoenix/margin-and-risk/entry-price-pnl.md -->

---

# Source: `phoenix/margin-and-risk/funding-rate.md`

**Original URL:** `https://docs.phoenix.trade/phoenix/margin-and-risk/funding-rate.md`

**Local path:** `phoenix/margin-and-risk/funding-rate.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Funding

> Funding rates keep perpetual prices aligned with spot prices through periodic payments between traders.

## How It Works

$$
\text{funding} = \left(\text{mark price} - \text{index price}\right) \times \text{rate}
$$

* **When $\text{mark price} > \text{index price}$:** Longs pay shorts (incentivizes selling, reduces perp price)
* **When $\text{mark price} < \text{index price}$:** Shorts pay longs (incentivizes buying, raises perp price)

## Settlement

* Funding accumulates continuously based on hourly snapshots and settles every 24 hours
* Pending funding (but unsettled) can affect account health/liquidation risk in real time
* The maximum funding rate is clamped per market. The percentage is dependent on mark price and can be found in the [Market Parameters](/phoenix/market-parameters).

When a trader interacts with the protocol, their pending funding is calculated as:

$$
\text{funding payment}
= \left(\text{current accumulator} - \text{snapshot accumulator}\right)
\times \text{position size}
$$

<!-- END SOURCE: phoenix/margin-and-risk/funding-rate.md -->

---

# Source: `phoenix/margin-and-risk/leverage-tiers.md`

**Original URL:** `https://docs.phoenix.trade/phoenix/margin-and-risk/leverage-tiers.md`

**Local path:** `phoenix/margin-and-risk/leverage-tiers.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Leverage Tiers

> How position size affects max leverage and margin

Phoenix uses market-specific leverage tiers.

As position size increases, the market may require more collateral for the same notional exposure. Smaller positions can receive higher max leverage. Larger positions may receive lower max leverage.

## How tiers affect margin

Initial margin is based on position size, mark price, and the max leverage available at that size:

$$
\text{initial margin} = \frac{\text{position notional}}{\text{maximum leverage}}
$$

If a fill increases your total position size into a lower-leverage range, Phoenix recalculates margin for the full resulting position, not only the newest fill.

Reducing position size can lower margin because:

* the notional position is smaller
* the remaining size may qualify for higher max leverage

## Resting orders

Risk-increasing resting limit orders can also reserve margin before they fill.

Phoenix evaluates whether resting bids or asks could increase exposure. If they can, the account may need additional collateral even though the orders have not executed yet.

Reduce-only orders do not add exposure.

## Live tier values

Leverage tiers are market-specific and can change.

Use [Market Parameters](/phoenix/market-parameters) for current tier values, including:

* max leverage
* max size
* limit order risk factor

<!-- END SOURCE: phoenix/margin-and-risk/leverage-tiers.md -->

---

# Source: `phoenix/margin-and-risk/liquidations.md`

**Original URL:** `https://docs.phoenix.trade/phoenix/margin-and-risk/liquidations.md`

**Local path:** `phoenix/margin-and-risk/liquidations.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Risk Tiers And Liquidation

> Risk classification and liquidation types

Phoenix classifies trader accounts by comparing effective collateral against margin thresholds.

As collateral falls, the account moves into progressively more severe risk tiers. Each tier gives the protocol stronger tools to reduce risk.

## Risk tiers

| Tier | Status               | Condition                                                                           | Action                                            |
| ---- | -------------------- | ----------------------------------------------------------------------------------- | ------------------------------------------------- |
| 0    | Low Risk             | $\text{effective collateral} \geq \text{initial margin}$                            | No immediate action                               |
| 1    | At Risk              | $\text{cancel margin} < \text{effective collateral} < \text{initial margin}$        | Monitored                                         |
| 2    | Cancellable          | $\text{maintenance margin} < \text{effective collateral} \leq \text{cancel margin}$ | Risk-increasing limit orders can be cancelled     |
| 3    | Liquidatable         | $\text{effective collateral} < \text{maintenance margin}$                           | Positions are eligible for market liquidation     |
| 4    | BackstopLiquidatable | $\text{effective collateral} < \text{backstop threshold}$                           | Backstop liquidation can transfer distressed risk |
| 5    | High Risk            | $\text{effective collateral} < \text{high-risk threshold}$                          | ADL eligible                                      |

## Liquidation types

### Market liquidation

For liquidatable accounts, Phoenix can reduce positions through market execution.

* executes against available liquidity
* can be partial
* must improve trader health or fully close the position
* respects market liquidation size limits

### Backstop liquidation

If market liquidation is not enough, Phoenix can transfer a distressed position to a backstop account.

* the backstop account absorbs the position
* the backstop account then unwinds the risk through normal market execution
* this path is used when orderbook liquidity is insufficient or the account is too distressed for ordinary liquidation

### ADL

ADL is the last-resort path.

* the losing position is matched against a profitable trader on the opposite side
* the highest-priority profitable trader is selected first
* the match closes or reduces both sides
* profitable traders affected by ADL may realize less profit than expected

<!-- END SOURCE: phoenix/margin-and-risk/liquidations.md -->

---

# Source: `phoenix/margin-and-risk/margin-math.md`

**Original URL:** `https://docs.phoenix.trade/phoenix/margin-and-risk/margin-math.md`

**Local path:** `phoenix/margin-and-risk/margin-math.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Margin Math

> How Phoenix calculates effective collateral, position margin, and limit-order margin

Phoenix evaluates each trader account as a single margin account. Positions, funding, unrealized PnL, and risk-increasing resting orders all feed into the same account health calculation.

Margin is the mechanism that lets traders take positions larger than their posted collateral while keeping the exchange solvent. A perp venue always has a long and a short side for the same open interest. When one side has unrealized profit, the other side has the matching liability. The risk system is designed to keep those liabilities collateralized and to make sure risky positions can be reduced before they create bad debt.

At a high level:

$$
\text{account health} = \text{effective collateral} - \text{margin requirement}
$$

If effective collateral falls through the risk thresholds, Phoenix can cancel risk-increasing orders, liquidate through the order book, escalate to backstop liquidation, and finally use ADL.

## Risk invariants

Phoenix margin is built around three practical invariants:

* long and short open interest must balance
* unrealized profits must be matched by liabilities elsewhere in the system
* those liabilities must remain collateralized

If a losing account can no longer cover the unrealized profit owed to the other side, the system has bad debt. Margin requirements, mark prices, funding, liquidation thresholds, and open interest controls all exist to reduce the probability of that outcome.

## Effective collateral

Effective collateral is the collateral value Phoenix uses for risk checks. It is similar to account equity, but Phoenix can discount positive unrealized PnL before treating it as usable collateral.

$$
\begin{aligned}
\text{effective collateral} ={}& \text{deposited collateral} \\
&+ \text{discounted positive unrealized PnL} \\
&+ \text{negative unrealized PnL} \\
&+ \text{unsettled funding}
\end{aligned}
$$

Important details:

* deposited collateral is the USDC collateral balance on the trader account
* positive unrealized PnL can be discounted by the market's `uPnL risk factor`
* negative unrealized PnL counts in full
* unsettled funding is included because it changes account risk before final settlement

Phoenix discounts positive uPnL to reduce the risk that manipulated or unstable mark prices create too much usable collateral. This matters more for volatile or lower-liquidity assets, where mark prices can be easier to move temporarily.

See [Market Parameters](/phoenix/market-parameters) for market-specific `uPnL risk factor` values.

## Mark price and PnL

Position value is based on mark price, not the last trade price.

$$
\text{unrealized PnL}
= \text{position size} \times \left(\text{mark price} - \text{entry price}\right)
$$

Using mark price for PnL, margin, and liquidation checks helps prevent a single trade or thin order book from pushing accounts into unsafe states. See [Mark Price](/phoenix/margin-and-risk/mark-price) for how Phoenix builds the mark.

## Position margin

For each market, Phoenix calculates margin from the absolute position size and the market's leverage tier.

$$
\text{position notional}
= \left|\text{position size}\right| \times \text{mark price}
$$

$$
\text{position margin}
= \frac{\text{position notional}}{\text{maximum leverage for size}}
$$

The leverage tier is selected from the resulting position size. A fill that moves the total position into a different tier can change margin for the whole resulting position, not only the newest fill.

## Limit-order margin

Risk-increasing resting limit orders can require margin before they fill.

Phoenix evaluates resting bids and resting asks separately for each market:

$$
M_{\text{market, initial}}
= M_{\text{position}} + \max\left(\Delta M_{\text{bid}},\ \Delta M_{\text{ask}}\right)
$$

This matters because a trader cannot fill every bid and every ask in the same market into the same final directional exposure. Phoenix therefore reserves margin for the worse side, not the sum of both sides.

The margin increase for each side is based on the hypothetical position if those non-reduce-only orders fill:

$$
\Delta M_{\text{side}}
= \left(M_{\text{post-fill}} - M_{\text{position}}\right) \times f_{\text{limit order}}
$$

Notes:

* reduce-only orders do not add margin
* orders that only reduce or close the current position do not add margin
* limit-order margin uses the market's mark price and size, not the order's limit price
* `limit_order_risk_factor` is market- and tier-specific
* limit-order margin is part of initial margin and can move an account toward the cancellation threshold before the order fills

See [Leverage Tiers](/phoenix/margin-and-risk/leverage-tiers) and [Market Parameters](/phoenix/market-parameters).

## Example portfolio

Assume a cross account has `12,000 USDC` deposited collateral and the following open positions:

| Market | Position        | Entry    | Mark     | Unrealized PnL |
| ------ | --------------- | -------- | -------- | -------------- |
| BTC    | Long `0.10 BTC` | `60,000` | `62,000` | `+200 USDC`    |
| SOL    | Short `100 SOL` | `150`    | `140`    | `+1,000 USDC`  |

Using the fallback market parameters for BTC and SOL, positive uPnL receives `100%` collateral credit for trading risk checks. Assume unsettled funding is `-50 USDC`.

$$
\begin{aligned}
\text{discounted positive unrealized PnL}
&= \left(200 + 1{,}000\right) \times 1.00 \\
&= 1{,}200.000000
\end{aligned}
$$

$$
\begin{aligned}
\text{effective collateral}
&= 12{,}000 + 1{,}200 - 50 \\
&= 13{,}150.000000\ \text{USDC}
\end{aligned}
$$

Now calculate position margin using the first leverage tier from `phoenix/market-parameters-fallback.json`. Values below include the quote-lot rounding used by the Rise margin calculator.

| Market | Position notional                | Max leverage | Position margin |
| ------ | -------------------------------- | ------------ | --------------- |
| BTC    | $0.10 \times 62{,}000 = 6{,}200$ | $20\times$   | $310.000000$    |
| SOL    | $100 \times 140 = 14{,}000$      | $15\times$   | $933.333334$    |

The account also has resting limit orders:

| Market | Resting orders             | Effect if filled                                       |
| ------ | -------------------------- | ------------------------------------------------------ |
| ETH    | Bid `2 ETH`, ask `1 ETH`   | No current ETH position, so either side opens exposure |
| SOL    | Bid `50 SOL`, ask `25 SOL` | The bid reduces the short; the ask increases the short |

Assume ETH mark price is `3,000` and SOL mark price is `140`. In the fallback parameters, ETH uses `20x` first-tier max leverage, SOL uses `15x`, and both markets use a `100%` first-tier `limit_order_risk_factor`.

In the calculations below, $M$ denotes margin and $\Delta M$ denotes an increase in margin.

For ETH:

$$
M_{\text{bid, initial}}
= \frac{2\ \text{ETH} \times 3{,}000}{20}
= 300.000000
$$

$$
M_{\text{bid, order}}
= 300.000000 \times 100\%
= 300.000000
$$

$$
M_{\text{ask, initial}}
= \frac{1\ \text{ETH} \times 3{,}000}{20}
= 150.000000
$$

$$
M_{\text{ask, order}}
= 150.000000 \times 100\%
= 150.000000
$$

$$
M_{\text{ETH, initial}}
= 0 + \max\left(300.000000, 150.000000\right)
= 300.000000
$$

For SOL:

$$
M_{\text{SOL, position}}
= \frac{100\ \text{SOL} \times 140}{15}
= 933.333334
$$

Buying $50\ \text{SOL}$ reduces the short from $100\ \text{SOL}$ to $50\ \text{SOL}$, so:

$$
M_{\text{bid, order}} = 0
$$

Selling $25\ \text{SOL}$ increases the short from $100\ \text{SOL}$ to $125\ \text{SOL}$, so:

$$
\begin{aligned}
M_{\text{post-ask}} &= \frac{125\ \text{SOL} \times 140}{15} = 1{,}166.666667 \\[0.75em]
\Delta M_{\text{ask}} &= 1{,}166.666667 - 933.333334 = 233.333333 \\[0.75em]
M_{\text{ask, order}} &= 233.333333 \times 100\% = 233.333333
\end{aligned}
$$

$$
M_{\text{SOL, initial}}
= 933.333334 + \max\left(0, 233.333333\right)
= 1{,}166.666667
$$

Total initial margin:

$$
\begin{aligned}
\text{BTC margin} &= 310.000000 \\[0.5em]
\text{SOL margin} &= 1{,}166.666667 \\[0.5em]
\text{ETH margin} &= 300.000000 \\[1em]
\text{total initial margin} &= 1{,}776.666667\ \text{USDC}
\end{aligned}
$$

The account is healthy because effective collateral is above initial margin:

$$
13{,}150.000000\ \text{effective collateral}
> 1{,}776.666667\ \text{initial margin}
$$

Using the fallback threshold factors for these markets, the account's risk thresholds would be:

| Threshold            | Calculation                                                                    | Result           |
| -------------------- | ------------------------------------------------------------------------------ | ---------------- |
| Cancel margin        | $310.000000 \times 70\% + 1{,}166.666667 \times 75\% + 300.000000 \times 70\%$ | $1{,}302.000000$ |
| Maintenance margin   | $\text{total initial margin} \times 50\%$                                      | $888.333333$     |
| Backstop requirement | $\text{total initial margin} \times 20\%$                                      | $355.333333$     |
| High-risk margin     | $\text{total initial margin} \times 10\%$                                      | $177.666666$     |

## Maintenance margin and other thresholds

Phoenix derives lower risk thresholds from initial margin using market-specific risk factors. These thresholds are the escalation ladder the protocol uses as an account becomes more dangerous to the system.

$$
\begin{aligned}
M_{\text{cancel}} &= M_{\text{initial}} \times f_{\text{cancel}} \\[0.5em]
M_{\text{maintenance}} &= M_{\text{initial}} \times f_{\text{maintenance}} \\[0.5em]
M_{\text{backstop}} &= M_{\text{initial}} \times f_{\text{backstop}} \\[0.5em]
M_{\text{high risk}} &= M_{\text{initial}} \times f_{\text{high risk}}
\end{aligned}
$$

Here, $M$ is the margin threshold and $f$ is its corresponding market-specific risk factor.

Initial margin is the opening and healthy-account requirement. Maintenance margin is the main liquidation threshold. If effective collateral falls below maintenance margin, the account can be liquidated.

The broader risk sequence is:

* below cancel margin: risk-increasing limit orders can be cancelled
* below maintenance margin: market liquidation can begin
* below backstop requirement: distressed positions can be transferred to a backstop account
* below high-risk margin: ADL can become available

See [Liquidations](/phoenix/margin-and-risk/liquidations) for the liquidation sequence.

## Liquidation estimates

Liquidation price is an estimate, not a fixed promise.

For a single position, Phoenix solves for the mark price where effective collateral would fall to the relevant maintenance requirement:

$$
C + U_{\text{other}} + q\left(P - P_{\text{entry}}\right) = M_{\text{other}} + \frac{f_{\text{maintenance}}\left|q\right|P}{L}
$$

Where:

* $C$ is collateral
* $U_{\text{other}}$ is the unrealized PnL contribution from other markets in the same cross account
* $q$ is signed position size
* $P$ is the estimated liquidation price
* $L$ is the active leverage tier
* $M_{\text{other}}$ is the maintenance margin from other markets in the same cross account
* $f_{\text{maintenance}}$ is the maintenance risk factor

This is why cross-margin liquidation prices can move when funding, mark prices, other positions, or resting orders change.

In a cross account, profitable positions in one market can support risk in another market. That is capital efficient, but it also means liquidation estimates are portfolio-aware. A BTC liquidation price can move because SOL PnL changed, funding settled, or a risk-increasing ETH order was placed.

See [Liquidations](/phoenix/margin-and-risk/liquidations) for the expanded formula and limit-order example.

<!-- END SOURCE: phoenix/margin-and-risk/margin-math.md -->

---

# Source: `phoenix/margin-and-risk/mark-price.md`

**Original URL:** `https://docs.phoenix.trade/phoenix/margin-and-risk/mark-price.md`

**Local path:** `phoenix/margin-and-risk/mark-price.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Mark Price

> Mark price determines position valuations for PnL and liquidation calculations.

## Calculation

Mark price is calculated as the median of three components:

1. **Adjusted Oracle Price:** Spot oracle price adjusted with a smoothed basis
2. **Book Price:** Median of best bid (highest price someone is willing to pay), best ask (lowest price someone is willing to sell at), and last trade on the Phoenix orderbook (most recent completed transaction price)
3. **Exchange Price:** Weighted median of perpetual prices from major external exchanges

## Design Goals

Phoenix was designed with the following core principles:

* Aim to closely track spot price movements for fair PnL and valuations
* Help protect against manipulation or distortions from temporary liquidity shocks on individual venues
* Seek to reduce the risk of unnecessary liquidation wicks due to short-term anomalies or low-liquidity events such as volatile market conditions, oracle delays, or network issues that may cause deviations

<!-- END SOURCE: phoenix/margin-and-risk/mark-price.md -->

---

# Source: `phoenix/margin-and-risk/risk-warning.md`

**Original URL:** `https://docs.phoenix.trade/phoenix/margin-and-risk/risk-warning.md`

**Local path:** `phoenix/margin-and-risk/risk-warning.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Risk Warning

> Important risk disclosures

## Overview

Trading perpetual futures involves substantial risk of loss. Always monitor your maintenance margin and account health to avoid liquidation. Use appropriate position sizing and stop losses, especially when using leverage.

## Smart Contract Risks

All smart contracts carry inherent risks. Vulnerabilities or exploits could result in loss of funds.

## Solana Network Risks

Network congestion can delay transactions, order execution, liquidations, and funding settlements. Solana network outages may temporarily prevent access to positions.

## Oracle Dependency Risks

Oracle price feeds can experience delays, gaps, or failures at times. Oracle outages could trigger unexpected liquidations or prevent timely position adjustments.

<!-- END SOURCE: phoenix/margin-and-risk/risk-warning.md -->

---

# Source: `phoenix/market-parameters.md`

**Original URL:** `https://docs.phoenix.trade/phoenix/market-parameters.md`

**Local path:** `phoenix/market-parameters.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Phoenix Market Parameters

> Phoenix Markets: Risk, Leverage, and Parameters

export const API_URL = "https://perp-api.phoenix.trade/exchange/markets";
export const FALLBACK_RETRY_MS = 10_000;
export const MARKET_ORDER = {
  SOL: 0,
  BTC: 1,
  ETH: 2
};

export const formatNumber = value => Number(value).toLocaleString("en-US");

export const toFixedDigits = decimals => {
  const digits = Number(decimals);
  if (!Number.isFinite(digits)) return 0;
  return Math.min(100, Math.max(0, Math.trunc(digits)));
};

export const baseLotsToBase = (baseLots, decimals) => Number(baseLots) / Math.pow(10, decimals);

export const formatBaseUnits = (baseLots, decimals) => {
  const value = baseLotsToBase(baseLots, decimals);
  const trimmed = parseFloat(value.toFixed(toFixedDigits(decimals)));
  if (trimmed >= 1000) return trimmed.toLocaleString("en-US");
  return String(trimmed);
};

export const formatBaseLotSize = decimals => (1 / Math.pow(10, decimals)).toFixed(toFixedDigits(decimals));

export const formatRiskPercent = rawValue => rawValue.toFixed(2) + "%";

export const formatTickSize = (tickSize, decimals) => {
  const price = tickSize * Math.pow(10, decimals - 6);
  return "$" + price.toFixed(6);
};

export const logoUriForMarket = market => (market.logoUri ?? market.metadata?.logoUri) ?? null;

export const AssetLogo = ({market, size = 20}) => {
  const logoUri = logoUriForMarket(market);
  if (!logoUri) return null;
  return <img src={logoUri} alt={`${market.symbol} logo`} width={size} height={size} style={{
    borderRadius: "50%",
    flex: "0 0 auto",
    objectFit: "contain",
    verticalAlign: "middle"
  }} />;
};

export const MarketLabel = ({market, logoSize = 20}) => <span style={{
  display: "inline-flex",
  alignItems: "center",
  gap: "8px"
}}>
    <AssetLogo market={market} size={logoSize} />
    <span>{market.symbol}</span>
  </span>;

export const fetchMarketConfigs = async () => {
  const res = await fetch(API_URL, {
    cache: "no-store"
  });
  if (!res.ok) throw new Error("API returned " + res.status + " for " + API_URL);
  const json = await res.json();
  if (!Array.isArray(json)) throw new Error("Unexpected response shape from " + API_URL);
  return json;
};

export const SummaryTable = ({markets}) => <table style={{
  width: "100%",
  tableLayout: "auto"
}}>
    <thead>
      <tr>
        <th style={{
  textAlign: "left"
}}>Market</th>
        <th style={{
  textAlign: "left"
}}>Base Lot Size</th>
        <th style={{
  textAlign: "left"
}}>Tick Size</th>
        <th style={{
  textAlign: "left"
}}>OI Cap (Base)</th>
      </tr>
    </thead>
    <tbody>
      {markets.map(m => <tr key={m.symbol}>
          <td><MarketLabel market={m} /></td>
          <td>{formatBaseLotSize(m.baseLotsDecimals)}</td>
          <td>{formatTickSize(m.tickSize, m.baseLotsDecimals)}</td>
          <td>
            {formatNumber(baseLotsToBase(m.openInterestCapBaseLots, m.baseLotsDecimals))}{" "}
            {m.symbol}
          </td>
        </tr>)}
    </tbody>
  </table>;

export const MarketSection = ({m}) => {
  const decimals = m.baseLotsDecimals;
  return <>
      <h2>
        <MarketLabel market={m} logoSize={28} />
      </h2>
      <h3>Market Parameters</h3>
      <p>Leverage tiers scale required collateral as position size grows.</p>
      <table>
        <thead>
          <tr>
            <th style={{
    textAlign: "left"
  }}>Field</th>
            <th style={{
    textAlign: "left"
  }}>Value</th>
          </tr>
        </thead>
        <tbody>
          <tr><td>Market</td><td><MarketLabel market={m} /></td></tr>
          <tr><td>Asset ID</td><td>{m.assetId}</td></tr>
          <tr><td>Base Lot Decimals</td><td>{decimals}</td></tr>
          <tr>
            <td>Minimum Trade Size (1 base lot)</td>
            <td>{formatBaseLotSize(decimals)} base units</td>
          </tr>
          <tr>
            <td>Tick Size</td>
            <td>{formatTickSize(m.tickSize, decimals)} per base unit</td>
          </tr>
          <tr><td>Isolated Only</td><td>{m.isolatedOnly ? "Yes" : "No"}</td></tr>
          <tr>
            <td>Open Interest Cap</td>
            <td>{formatNumber(baseLotsToBase(m.openInterestCapBaseLots, decimals))} {m.symbol}</td>
          </tr>
          <tr>
            <td>Max Liquidation Size</td>
            <td>{formatBaseUnits(m.maxLiquidationSizeBaseLots, decimals)} {m.symbol}</td>
          </tr>
          <tr><td>Funding Period</td><td>{formatNumber(m.fundingPeriodSeconds)} seconds</td></tr>
          <tr><td>Funding Interval</td><td>{formatNumber(m.fundingIntervalSeconds)} seconds</td></tr>
          <tr>
            <td>Max Funding Rate per Period</td>
            <td>
              {formatRiskPercent(m.maxFundingRatePerIntervalPercentage * 24)} (based on current
              mark price)
            </td>
          </tr>
          <tr>
            <td>Max Funding Rate per Interval</td>
            <td>
              {formatRiskPercent(m.maxFundingRatePerIntervalPercentage)} (based on current mark
              price)
            </td>
          </tr>
          <tr><td>Maintenance Risk Factor</td><td>{formatRiskPercent(m.riskFactors.maintenance)}</td></tr>
          <tr><td>Backstop Risk Factor</td><td>{formatRiskPercent(m.riskFactors.backstop)}</td></tr>
          <tr><td>High Risk Factor</td><td>{formatRiskPercent(m.riskFactors.highRisk)}</td></tr>
          <tr><td>Cancel Order Risk Factor</td><td>{formatRiskPercent(m.riskFactors.cancelOrder)}</td></tr>
          <tr><td>UPnL Risk (Trading)</td><td>{formatRiskPercent(m.riskFactors.upnl)}</td></tr>
          <tr><td>UPnL Risk (Withdrawals)</td><td>{formatRiskPercent(m.riskFactors.upnlForWithdrawals)}</td></tr>
          <tr><td>Maker Fee</td><td>{formatRiskPercent(m.makerFee * 100)}</td></tr>
          <tr><td>Taker Fee</td><td>{formatRiskPercent(m.takerFee * 100)}</td></tr>
          <tr><td>Market Status</td><td>{m.marketStatus}</td></tr>
          <tr><td>Market Account</td><td><code>{m.marketPubkey}</code></td></tr>
          <tr><td>Spline Account</td><td><code>{m.splinePubkey}</code></td></tr>
        </tbody>
      </table>

      <h3>Leverage Tiers</h3>
      <p>Leverage tiers scale required collateral as position size grows.</p>
      <table>
        <thead>
          <tr>
            <th style={{
    textAlign: "left"
  }}>Tier</th>
            <th style={{
    textAlign: "left"
  }}>Max Leverage</th>
            <th style={{
    textAlign: "left"
  }}>Max Size (Base Units)</th>
            <th style={{
    textAlign: "left"
  }}>Limit Order Risk</th>
          </tr>
        </thead>
        <tbody>
          {m.leverageTiers.map((tier, i) => <tr key={i}>
              <td>{i + 1}</td>
              <td>{tier.maxLeverage}x</td>
              <td>{formatBaseUnits(tier.maxSizeBaseLots, decimals)} {m.symbol}</td>
              <td>{formatRiskPercent(tier.limitOrderRiskFactor)}</td>
            </tr>)}
        </tbody>
      </table>
    </>;
};

export const MarketParametersPage = () => {
  const [data, setData] = useState(null);
  const [error, setError] = useState(null);
  const [loading, setLoading] = useState(true);
  const [usingFallback, setUsingFallback] = useState(false);
  useEffect(() => {
    let cancelled = false;
    let retryTimeout = null;
    const loadMarketConfigs = async () => {
      try {
        const json = await fetchMarketConfigs();
        if (cancelled) return;
        setData(json);
        setError(null);
        setUsingFallback(false);
        setLoading(false);
      } catch (err) {
        if (cancelled) return;
        setData(fallbackMarkets);
        setError(err.message);
        setUsingFallback(true);
        setLoading(false);
        retryTimeout = setTimeout(loadMarketConfigs, FALLBACK_RETRY_MS);
      }
    };
    loadMarketConfigs();
    return () => {
      cancelled = true;
      if (retryTimeout) clearTimeout(retryTimeout);
    };
  }, []);
  if (loading) {
    return <div style={{
      display: "flex",
      alignItems: "center",
      gap: "8px",
      padding: "24px 0"
    }}>
        <svg width="16" height="16" viewBox="0 0 24 24" style={{
      animation: "spin 1s linear infinite"
    }}>
          <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
          <circle cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="3" fill="none" opacity="0.2" />
          <path d="M12 2a10 10 0 0 1 10 10" stroke="currentColor" strokeWidth="3" fill="none" strokeLinecap="round" />
        </svg>
        <span>Loading market parameters…</span>
      </div>;
  }
  if (error && !data) return <p>Failed to load market parameters: {error}</p>;
  const markets = [...data].filter(market => market.marketStatus === "active").sort((a, b) => {
    const orderDiff = (MARKET_ORDER[a.symbol] ?? 99) - (MARKET_ORDER[b.symbol] ?? 99);
    if (orderDiff !== 0) return orderDiff;
    return a.symbol.localeCompare(b.symbol);
  });
  return <>
      {usingFallback ? <p style={{
    padding: "12px 16px",
    border: "1px solid rgba(255, 160, 110, 0.35)",
    borderRadius: "12px",
    background: "rgba(255, 160, 110, 0.08)"
  }}>
          Live market parameters are temporarily unavailable. Showing fallback values and retrying.
        </p> : null}
      <h2>Summary</h2>
      <SummaryTable markets={markets} />
      {markets.map(m => <MarketSection key={m.symbol} m={m} />)}
    </>;
};

<MarketParametersPage />

<!-- END SOURCE: phoenix/market-parameters.md -->

---

# Source: `phoenix/matching-engine/fees.md`

**Original URL:** `https://docs.phoenix.trade/phoenix/matching-engine/fees.md`

**Local path:** `phoenix/matching-engine/fees.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Fees

> Trading fees, liquidation fees, referral discounts, and Flight builder fees

Phoenix charges trading fees on matched notional.

## Base trading fees

* taker fee: $3.5\ \text{bps}$ ($0.035\%$)
* maker fee: $0.5\ \text{bps}$ ($0.005\%$)

Examples:

$$
\begin{aligned}
\text{taker fee} &= 10{,}000 \times 0.035\% = 3.50\ \text{USDC} \\[0.5em]
\text{maker fee} &= 10{,}000 \times 0.005\% = 0.50\ \text{USDC}
\end{aligned}
$$

## Maker vs taker

* you are a maker when your order provides resting liquidity
* you are a taker when your order removes existing liquidity

A limit order can be either:

* taker on the crossed portion
* maker on any remainder that rests

## What else can affect your all-in cost

### Referral discount

Referral codes can reduce user trading fees. See [Referral Program](/incentive-programs/referral-program).

### Flight builder fee

If you route an order through Flight, the builder fee is additive to exchange trading fees. See [Builder Codes](/builder-codes).

<!-- END SOURCE: phoenix/matching-engine/fees.md -->

---

# Source: `phoenix/matching-engine/fifo-order-book.md`

**Original URL:** `https://docs.phoenix.trade/phoenix/matching-engine/fifo-order-book.md`

**Local path:** `phoenix/matching-engine/fifo-order-book.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# FIFO Order Book

> Price-time priority and user-facing orderbook behavior

Phoenix uses a FIFO order book for normal resting orders.

FIFO means:

* better price fills first
* at the same price, older resting liquidity fills before newer resting liquidity

## What a resting order stores

A resting FIFO order stores:

* the maker trader position
* initial and remaining base size
* order flags
* optional conditional order index
* optional slot-based expiry
* initial slot

## Capacity and order limits

Phoenix enforces both per-trader open-order limits and market-level orderbook capacity.

* A trader can have up to `64` resting bid limit orders per market.
* A trader can have up to `64` resting ask limit orders per market.
* The per-trader limit is per market and per side, so one trader can have up to `128` resting FIFO orders on one market: `64` bids and `64` asks.
* IOC and other take-only orders do not count toward this cap because they do not rest on the book.

## Order packet families

At the protocol layer, the core order packet kinds are:

| Packet kind         | User-facing mental model                | Rests on book                    |
| ------------------- | --------------------------------------- | -------------------------------- |
| `PostOnly`          | maker-only limit order                  | Yes                              |
| `Limit`             | standard limit order                    | Yes, after any immediate matches |
| `ImmediateOrCancel` | IOC, market, or FOK depending on fields | No                               |

## Important fields

### `price_in_ticks`

The order price in native book ticks.

### `num_base_lots`

The total base size to fill or rest.

### `last_valid_slot`

Optional expiry slot for the order.

### `match_limit`

Optional cap on how many resting orders an incoming order can match against.

### `cancel_existing`

Advanced behavior that lets some order paths cancel conflicting resting orders instead of failing a margin check immediately.

## Maker vs taker behavior

* A resting order that provides liquidity is maker flow.
* An incoming aggressive order that removes resting liquidity is taker flow.
* A standard limit order can act as either, depending on whether it crosses the book.

## Post-only behavior

Post-only orders are for adding liquidity only.

* If they would immediately trade, they do not execute as takers.
* Depending on configuration, they may slide to a valid resting price rather than cross.

## Expiry and live L2 views

FIFO order expiry is slot-based. An expired order can remain in the raw book until a matching or maintenance path touches it. Consumers that need a live L2 view should filter expired FIFO orders before aggregating by price.

<!-- END SOURCE: phoenix/matching-engine/fifo-order-book.md -->

---

# Source: `phoenix/matching-engine/matching-engine.md`

**Original URL:** `https://docs.phoenix.trade/phoenix/matching-engine/matching-engine.md`

**Local path:** `phoenix/matching-engine/matching-engine.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Matching Engine

> How Phoenix matches orders across the FIFO order book and spline liquidity

Phoenix builds one matching view from two liquidity sources.

## Liquidity surfaces

### FIFO order book

The FIFO order book is the standard limit-order book. Orders fill by price first, then by time priority within the same price level.

### Spline liquidity

Spline liquidity is virtual liquidity published by authorized spline traders. A spline trader quotes a mid price and bid/ask regions around that mid. The engine materializes those regions into visible price levels when it builds the combined book.

For each interaction, the engine builds a combined view over both sources. It loads the FIFO bid and ask maps, loads eligible splines, applies spline risk caps and current-slot fill state, and then matches against the best available price across the combined book.

## Fast quote updates

Splines are designed to let market makers move liquidity with very low compute overhead.

On a traditional FIFO book, a market maker that wants to move an entire quote ladder has to place and cancel many individual limit orders. A multi-limit-order transaction can cost up to roughly `300,000` compute units.

With spline liquidity, the market maker can update the spline's reference price and move the quoted regions around that price. A spline oracle update costs around `600` compute units.

That compute asymmetry matters for market structure. When external prices move, stale on-chain quotes are vulnerable to toxic taker flow. Lower-cost spline updates let market makers refresh quotes more quickly and more frequently, which helps them quote tighter spreads without pricing in as much stale-quote risk.

This does not guarantee transaction ordering. It gives quote updates a much smaller compute footprint, which improves the economics and latency profile of keeping Phoenix liquidity close to the current market. Spline traders can also attach their own sequence numbers to updates, so stale updates can be rejected if validators reorder multiple spline updates from the same trader.

## Execution priority

The engine uses price priority across FIFO and spline liquidity:

* for an incoming buy, the lower ask fills first
* for an incoming sell, the higher bid fills first
* if FIFO and spline liquidity are available at the same price, spline liquidity fills first

Equal-price spline priority does not let deeper spline liquidity skip better FIFO prices. When splines win priority at a price, the engine only matches spline levels up to the current FIFO boundary before returning to FIFO.

## High-level flow

1. A user submits an order packet.
2. The engine validates price, size, flags, expiry, and self-trade behavior. If the order can rest, it also checks the trader's per-side open-order cap before placement.
3. The engine builds the combined matching view.
4. The order matches eligible FIFO or spline liquidity until the order is filled, price-limited, budget-limited, or match-limited.
5. Any remaining size rests on the FIFO book, expires, or is discarded depending on the order packet.

## What changes fill behavior

* whether the incoming order crosses the combined BBO
* `self_trade_behavior`
* `match_limit`
* post-only slide or reject behavior
* execution price band filtering
* current spline risk counters
* whether a spline-only cross has been resolved into slide prices

<!-- END SOURCE: phoenix/matching-engine/matching-engine.md -->

---

# Source: `phoenix/matching-engine/order-types.md`

**Original URL:** `https://docs.phoenix.trade/phoenix/matching-engine/order-types.md`

**Local path:** `phoenix/matching-engine/order-types.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Order Types

> Phoenix order types, matching behavior, and advanced order controls

Phoenix exposes a small set of core order packet families and then layers user-facing controls on top.

## Core order families

| Public name     | Protocol mental model                          | Rests on book | Notes                                           |
| --------------- | ---------------------------------------------- | ------------- | ----------------------------------------------- |
| Market order    | IOC with no explicit limit price               | No            | Pure taker flow                                 |
| IOC             | Immediate-or-cancel with an optional price cap | No            | Cancels any unfilled remainder                  |
| FOK             | IOC with minimum fill constraints              | No            | Entire minimum must fill or the order is voided |
| Limit order     | Cross if possible, then rest                   | Yes           | Can be maker or taker depending on price        |
| Post-only order | Maker-only limit order                         | Yes           | Will not take existing liquidity                |

## User-facing modifiers

### Reduce only

Reduce-only orders can only decrease existing exposure.

### Post only

Post-only orders are used to add liquidity only. They do not execute immediately as takers.

### Take profit / stop loss

These are trigger orders evaluated against mark price. See [Take Profit And Stop Loss](/phoenix/matching-engine/take-profit-stop-loss).

### Expiration

Order packets can carry a `last_valid_slot`, which acts like an expiry.

### Match limit

Advanced callers can set `match_limit` to cap how many resting FIFO orders or spline price levels an incoming order may consume.

This is separate from the open-order cap. `match_limit` controls taker-side matching work for one incoming order. The open-order cap controls how many resting limit orders a trader may already have on one market side.

For FIFO matching, each maker order consumes one `match_limit` unit. For spline matching, one spline price level consumes one `match_limit` unit even if multiple splines participate at that level.

### Cancel existing

Some order flows can request cancellation of conflicting resting orders instead of failing a margin check immediately.

## How to think about order choice

### Use market / IOC when

* speed matters most
* you are willing to take available liquidity

### Use limit when

* you need a price bound
* you are okay with resting if not fully filled

### Use post-only when

* you explicitly want maker behavior
* you do not want an aggressive fill

## Self-trade behavior

Phoenix also lets the matching engine choose how to handle self-crosses.

At the protocol layer, the supported behaviors are:

* `Abort`
* `CancelProvide`
* `DecrementTake`

Standard limit and IOC builders default to `CancelProvide` unless a caller overrides it.

<!-- END SOURCE: phoenix/matching-engine/order-types.md -->

---

# Source: `phoenix/matching-engine/self-trade-prevention.md`

**Original URL:** `https://docs.phoenix.trade/phoenix/matching-engine/self-trade-prevention.md`

**Local path:** `phoenix/matching-engine/self-trade-prevention.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Self-Trade Prevention

> How Phoenix handles orders that would match against your own liquidity

Phoenix has explicit self-trade prevention modes at the matching-engine layer.

## Available behaviors

| Behavior        | What happens                                                    |
| --------------- | --------------------------------------------------------------- |
| `Abort`         | Reject the self-cross instead of matching it                    |
| `CancelProvide` | Cancel the resting provide-side order on the self-cross         |
| `DecrementTake` | Reduce the incoming take-side quantity by the self-cross amount |

## Default behavior

For the core limit and IOC builders, the protocol defaults to `CancelProvide` unless the caller explicitly chooses another behavior.

Some specialized flows use stricter defaults. For example:

* many SDK examples use `Abort`
* TP/SL-triggered orders are designed to avoid accidental self-matching

## Why this matters

Without STP, a trader could:

* fill against their own resting order
* pay fees for a meaningless self-match
* distort fill history and realized PnL

## User-facing outcomes

If a self-cross is detected, you may see:

* an order reject
* a resting order cancellation
* an incoming order that fills less than its original requested size

Those are not random outcomes. They depend on the selected STP mode.

<!-- END SOURCE: phoenix/matching-engine/self-trade-prevention.md -->

---

# Source: `phoenix/matching-engine/spline-liquidity.md`

**Original URL:** `https://docs.phoenix.trade/phoenix/matching-engine/spline-liquidity.md`

**Local path:** `phoenix/matching-engine/spline-liquidity.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Spline Liquidity

> How spline traders quote and fill liquidity on Phoenix

Spline liquidity is a Phoenix-specific liquidity source for quoting a shaped curve without placing one FIFO order at every price level.

A spline trader publishes a mid price and bid/ask regions. The matching engine materializes those regions into visible price levels when it builds the combined book.

## Spline data model

Each spline has:

* a `mid_price`
* bid regions below mid
* ask regions above mid
* sequence ordering
* current fill state
* risk-limited bid and ask capacity

Each region is defined by:

* `start_offset`
* `end_offset`
* `density`
* optional top-level hidden take size
* lifespan / expiry

Region offsets are tick distances from the spline mid:

* bid offset $x$ maps to $\text{mid price} - x$
* ask offset $x$ maps to $\text{mid price} + x$
* `density` is the base-lot size available per tick in the region

For example, a spline with mid `100`, bid region `[1, 3)`, and density `5` exposes:

* `5` lots at `99`
* `5` lots at `98`

An ask region `[1, 3)` on the same spline exposes:

* `5` lots at `101`
* `5` lots at `102`

## How spline levels become visible

A spline level is eligible for the book when:

* the spline is active
* the spline has a nonzero mid price
* the region has not expired
* the region still has unfilled size
* the spline trader has remaining risk capacity on that side

The visible level is the first unfilled tick in the active region. If a tick is partially filled, only the remaining size at that tick is displayed. Once a tick is fully consumed, the next tick in the region becomes visible.

Spline liquidity is therefore not a static list of orders. It is derived from the spline's mid price, region definitions, region fill state, expiry, and current risk counters.

## Risk caps

Before matching, Phoenix caps each spline's available bid and ask size using the spline trader's current risk state.

The cap accounts for:

* current margin and position state
* `leverage_decrease_in_bps`
* optional max position size limits
* the current risk action

After each spline fill, the in-memory risk counter is decremented. This prevents one instruction from consuming more spline liquidity than the trader can support across multiple price levels.

## Uncrossing and slide prices

Spline liquidity from different traders can cross when their mids and regions overlap. Phoenix resolves this before rendering or matching spline-only liquidity.

The uncross path:

1. Matches crossed splines against FIFO liquidity first when the spline crosses the explicit book.
2. Computes slide prices for remaining crossed spline-only liquidity.
3. Displays and matches more aggressive spline bids at the slide bid.
4. Displays and matches more aggressive spline asks at the slide ask.

Slide prices make the remaining spline book non-crossing without writing synthetic orders into the FIFO book.

## Pro-rata fills

At a single spline price level, FIFO price-time priority does not apply across spline traders. Phoenix allocates the fill pro-rata by available size:

$$
\text{allocation} = \left\lfloor \frac{\text{available lots} \times \text{fill size}}{\text{total available}} \right\rfloor
$$

Spline sequence number is still used for deterministic iteration and dust handling. It is not the primary priority rule.

Important execution consequences:

* one spline price level consumes one `match_limit` unit, even if multiple splines participate
* one FIFO maker order consumes one `match_limit` unit
* spline liquidity at the same price fills before FIFO liquidity
* worse spline prices do not skip better FIFO prices

## Hidden take size

`top_level_hidden_take_size` is not displayed as normal resting spline liquidity for incoming takers.

It is used when a spline itself takes FIFO liquidity during uncrossing. In that path, Phoenix can consume visible size at the current top tick, then hidden take size at that same tick, then visible size through deeper ticks.

<!-- END SOURCE: phoenix/matching-engine/spline-liquidity.md -->

---

# Source: `phoenix/matching-engine/take-profit-stop-loss.md`

**Original URL:** `https://docs.phoenix.trade/phoenix/matching-engine/take-profit-stop-loss.md`

**Local path:** `phoenix/matching-engine/take-profit-stop-loss.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Take Profit and Stop Loss

> Attach conditional orders to positions or limit orders to close at a trigger price

Take Profit (TP) and Stop Loss (SL) are conditional, reduce-only orders that execute when the market's mark price crosses a trigger price you specify. You can attach TP/SL to an open position or to a resting limit order.

## When triggers fire

Triggers are evaluated against the market's [mark price](/phoenix/margin-and-risk/mark-price).

| Position | Take Profit fires when | Stop Loss fires when  |
| -------- | ---------------------- | --------------------- |
| Long     | Mark price ≥ TP price  | Mark price ≤ SL price |
| Short    | Mark price ≤ TP price  | Mark price ≥ SL price |

## Execution style

When a trigger fires, the conditional order is placed on the book in one of two modes:

* **Market:** Default option. Submitted as an immediate-or-cancel (IOC), all-or-none (AON) order. By default, these have the following slippage tolerances:

  * Take Profit: 0.25%
  * Stop Loss: 10%

  You can edit these values in settings.
* **Limit:** Submitted as a standard limit order at an execution price you specify. If your limit price doesn't cross the book, the order rests until it fills or is cancelled.

<Note>During real-world asset internal pricing periods, TP/SL market order slippage and limit order execution prices are clamped to the price bound limits.</Note>

TP/SL orders are always reduce-only.

## Where to place TP/SL

### On a new order

From the order form, enable the TP/SL toggle to attach conditional legs to a market or limit order. You can set each leg by trigger price, by gain/loss percentage, or by dollar amount — the inputs stay in sync. These will be placed as TP/SL [market orders](#execution-style).

### On an open position

Open the positions table and click the TP/SL cell for the position you want to bracket. The edit modal lets you set or update both legs independently. When triggered, the order executes for your full position size at that moment.

### On a resting limit order

From the open orders table, open a limit order and attach TP/SL in the edit modal. Once attached, the TP and SL appear as child rows beneath the parent limit order and share the parent's size.

## Limit order TP/SL lifecycle

A TP/SL attached to a limit order is linked to its parent while the parent rests on the book.

* **Partial fill of the parent:** The attached TP/SL's fillable size updates to match the parent's filled amount. The trigger, if breached, executes up to the filled amount.
* **Full fill of the parent:** The TP/SL detaches from the parent and becomes an **orphaned** conditional order. Orphaned orders remain active and can execute up to their full size when triggered.
* **Parent cancelled:** All attached TP/SL children are cancelled with the parent.
* **Cancel all:** Cancels all open orders, including attached children and orphans.

You can edit or cancel a child TP/SL directly without touching the parent.

### Position flips

Closing a position and opening the opposite side invalidates all active TP/SL on that market, including both position-based orders and any orphans or attached children. You'll need to reattach TP/SL after flipping.

## Chart controls

TP/SL orders render as lines on the trading chart. You can edit or cancel them directly from the chart.

## Limitations

* TP/SL cannot be attached to reduce-only limit orders.
* Self-trade prevention applies on execution.
* A trader account supports a bounded number of active conditional orders per market.

<!-- END SOURCE: phoenix/matching-engine/take-profit-stop-loss.md -->

---

# Source: `phoenix/perpetual-futures.md`

**Original URL:** `https://docs.phoenix.trade/phoenix/perpetual-futures.md`

**Local path:** `phoenix/perpetual-futures.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Perpetual Futures

> How Phoenix perpetual futures work, what makes them different from spot, and where each risk mechanism fits

Perpetual futures are leveraged contracts that track an underlying asset without an expiry date.

Phoenix uses them to let traders:

* go long or short with posted collateral
* trade against both resting limit orders and fast-updating spline liquidity
* keep positions open indefinitely as long as margin stays healthy

## What makes a perp different from spot

In spot trading, you buy or sell the asset itself.

In perps, you trade synthetic exposure instead:

* PnL is driven by mark-price changes versus your entry price
* leverage is created by posting only a fraction of notional as collateral
* funding transfers value between longs and shorts to keep the market anchored
* liquidation rules enforce solvency when collateral is no longer sufficient

## The core moving parts

### Entry price and PnL

Every position has an effective entry price and then accumulates unrealized PnL as mark price moves.

See [Entry Price And PnL](/phoenix/margin-and-risk/entry-price-pnl).

### Mark price

Phoenix uses a robust mark price instead of the last trade price. It combines an adjusted oracle price, Phoenix order book data, and external perpetual prices so risk checks are anchored to a fair market estimate rather than a single venue or transient fill

See [Mark Price](/phoenix/margin-and-risk/mark-price).

### Funding

Because perps have no expiry, funding is the mechanism that keeps the perp aligned with the underlying market over time.

See [Funding Rate](/phoenix/margin-and-risk/funding-rate).

### Margin

Margin lets traders take larger positions than their posted collateral alone would otherwise support, while accepting liquidation risk if losses, funding, or changing risk requirements erode the account’s collateral.

See [Accounts](/phoenix/collateral-and-accounts/accounts) and [Margin Math](/phoenix/margin-and-risk/margin-math).

### Liquidations and ADL

If effective collateral falls below maintenance margin, Phoenix first attempts normal liquidation through the order book after cancelling risk-increasing open orders. If the account remains underwater, liquidation can escalate to a backstop transfer, and finally ADL can be used last to close the remaining risk against profitable traders on the opposite side to preserve the exchange's health.

See [Liquidations](/phoenix/margin-and-risk/liquidations).

## How to navigate the docs from here

* Start with [Accounts](/phoenix/collateral-and-accounts/accounts) if you need the wallet and trader-account model
* Read [Matching Engine](/phoenix/matching-engine/matching-engine) if you want to understand fills
* Read [Margin Math](/phoenix/margin-and-risk/margin-math) if you want to understand liquidation risk
* Use [Market Parameters](/phoenix/market-parameters) once the earlier concepts are familiar

<!-- END SOURCE: phoenix/perpetual-futures.md -->

---

# Source: `phoenix/real-world-assets/impact-pricing.md`

**Original URL:** `https://docs.phoenix.trade/phoenix/real-world-assets/impact-pricing.md`

**Local path:** `phoenix/real-world-assets/impact-pricing.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Impact Pricing

> How Phoenix uses order book impact to price real-world asset perps when external markets are closed

Impact pricing is Phoenix's internal pricing mechanism for real-world asset perp markets when external markets are closed.

It uses Phoenix order book liquidity to move the last known external oracle price gradually, instead of freezing the index price until the next external session.

## Book Impact

With previous oracle price $S$, Phoenix calculates the price impact of crossing a configured notional amount against the order book:

$$
P_{\text{impact}} = \max\left(P_{\text{bid}} - S, 0\right) - \max\left(S - P_{\text{ask}}, 0\right)
$$

where $P_{\text{bid}}$ and $P_{\text{ask}}$ are the average execution prices for selling and buying $1{,}000\ \text{USD}$ notional, respectively. The configured notional is currently $1{,}000\ \text{USD}$ and is subject to change.

If one side of the book does not have enough depth for the configured notional, that side contributes `0` to the impact calculation.

## EMA Update

Phoenix adds the book impact price to the previous oracle price to form a target price:

$$
P_{\text{target}} = S + P_{\text{impact}}
$$

The internal oracle then moves toward the target using a continuous-time EMA. The default EMA time constant is 1 hour.

## Bounds

For real-world asset markets in internal pricing periods, Phoenix applies the market's configured price bounds before emitting the internal price.

The bound is set around the last known external index price. For example, a market with 20x maximum leverage has a 5% bound, while a market with 25x maximum leverage has a 4% bound.

<!-- END SOURCE: phoenix/real-world-assets/impact-pricing.md -->

---

# Source: `phoenix/real-world-assets/index-price.md`

**Original URL:** `https://docs.phoenix.trade/phoenix/real-world-assets/index-price.md`

**Local path:** `phoenix/real-world-assets/index-price.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Index Price

> How commodity and equity perp index prices update across external and internal pricing windows

## Oracle Prices

Phoenix real-world asset markets are perpetual futures. The sections below describe index pricing for commodity perps and equity perps.

Oracle prices for commodity perps and equity perps are denoted in USD. Phoenix does not apply a USDC/USD spot-price correction to these oracle prices.

Phoenix uses a number of independent data providers for external real-world asset prices.

## Commodity Perps

#### Market Hours

Commodity perp oracle prices are externally sourced during the following market schedule:

| Session            | Time                                            | Status   |
| ------------------ | ----------------------------------------------- | -------- |
| Sunday open        | 6:00 PM - 12:00 AM ET                           | External |
| Monday - Thursday  | 12:00 AM - 5:00 PM ET and 6:00 PM - 12:00 AM ET | External |
| Daily market break | 5:00 PM - 6:00 PM ET                            | Internal |
| Friday             | 12:00 AM - 5:00 PM ET                           | External |

During these market hours, commodity index prices are derived directly from liquid spot and futures markets.

Market hours are subject to exchange holidays and early closes.

#### After Hours

When traditional commodity markets are closed, the index price updates using [Impact Pricing](/phoenix/real-world-assets/impact-pricing), Phoenix's internal order-book based pricing mechanism.

#### Commodities Futures Rolling

Some commodity perp markets derive their pricing from futures contracts during market hours. Because futures contracts expire, the index price handles "rolling" the pricing to the next active futures contract via a 5-day roll between the 5th and 10th business days of the month.

During the roll period, 20% of the weight is shifted from the current contract to the next active contract at 5:30 ET every business day until the current contract is at 0% weight. Note that this happens during after hours, as markets are closed between 5 and 6 ET.

Currently this applies to the WTIOIL and COPPER markets.

| Business Day | Current Contract Weight | Next Active Contract Weight |
| ------------ | ----------------------- | --------------------------- |
| 5            | 100%                    | 0%                          |
| 6            | 80%                     | 20%                         |
| 7            | 60%                     | 40%                         |
| 8            | 40%                     | 60%                         |
| 9            | 20%                     | 80%                         |
| 10           | 0%                      | 100%                        |

## Equity Perps

#### Trading Hours

Equity perp oracle prices are externally sourced 24/5, from Sunday 8:00 PM ET to Friday 8:00 PM ET.

Phoenix achieves this by aggregating across the following equity-market sessions:

| Session     | Time                 |
| ----------- | -------------------- |
| Pre-market  | 4:00 AM - 9:30 AM ET |
| Market      | 9:30 AM - 4:00 PM ET |
| Post-market | 4:00 PM - 8:00 PM ET |
| Overnight   | 8:00 PM - 4:00 AM ET |

#### Market Closed

When the 24/5 external pricing window is closed, the index price updates using [Impact Pricing](/phoenix/real-world-assets/impact-pricing), Phoenix's internal order-book based pricing mechanism.

#### Futures Rolling

Equity markets do not use futures rolling mechanics.

## Transitions

When transitioning from an internal pricing period to an external pricing window, the index price instantly snaps to the new external market price. When transitioning from an external pricing window to an internal pricing period, the last known external price becomes the base value for internal pricing.

<!-- END SOURCE: phoenix/real-world-assets/index-price.md -->

---

# Source: `phoenix/real-world-assets/mark-price-and-bounds.md`

**Original URL:** `https://docs.phoenix.trade/phoenix/real-world-assets/mark-price-and-bounds.md`

**Local path:** `phoenix/real-world-assets/mark-price-and-bounds.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Mark Price And Bounds

> How mark price and closed-period price bounds work for real-world asset perps

## Mark Price

Commodity perps and equity perps use the same mark-price construction.

Just as in traditional crypto assets, the Mark Price is calculated as the median of three components.

1. **Adjusted Oracle Price:** Spot oracle price adjusted with a smoothed basis
2. **Book Price:** Median of best bid, best ask, and last trade on the Phoenix orderbook
3. **Exchange Price:** The spot oracle price plus a 375 slot EMA of the difference between the book mid price and the spot oracle price. During internal pricing periods, the EMA is taken over 9000 slots.

The third component differs from traditional crypto assets: it replaces the external perp exchange price. The oracle price can either be derived from external sources during external pricing windows or internal pricing during closed periods.

## Price Bounds

During internal pricing periods, price movements on Phoenix are capped within a range to protect users against oracle price manipulation in lower liquidity hours. The cap is set 1/(max leverage) away from the last known external market price.

For example, a market with 20x maximum leverage has a 5% bound, while a market with 25x maximum leverage has a 4% bound. The matching engine does not allow trades to execute outside of the price bounds.

<!-- END SOURCE: phoenix/real-world-assets/mark-price-and-bounds.md -->

---

# Source: `phoenix/real-world-assets/market-specs.md`

**Original URL:** `https://docs.phoenix.trade/phoenix/real-world-assets/market-specs.md`

**Local path:** `phoenix/real-world-assets/market-specs.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Market Specs

> Leverage, underlying references, price bounds, and margin modes for commodity and equity perps

For open interest caps and the full live parameter set, see [Market Parameters](/phoenix/market-parameters).

## Commodity Perps

| Market | Leverage | Closed-Period Price Bound | Underlying | Margin   |
| :----- | :------- | :------------------------ | :--------- | :------- |
| GOLD   | 25x      | 4%                        | XAU/USD    | Isolated |
| SILVER | 25x      | 4%                        | XAG/USD    | Isolated |
| COPPER | 20x      | 5%                        | HG/USD     | Isolated |
| WTIOIL | 20x      | 5%                        | M6/USD     | Isolated |

## Equity Perps

| Market | Leverage | Closed-Period Price Bound | Underlying | Margin   |
| :----- | :------- | :------------------------ | :--------- | :------- |
| AAPL   | 20x      | 5%                        | AAPL/USD   | Isolated |
| AMD    | 10x      | 10%                       | AMD/USD    | Cross    |
| AMZN   | 20x      | 5%                        | AMZN/USD   | Cross    |
| CRWV   | 10x      | 10%                       | CRWV/USD   | Cross    |
| GOOGL  | 20x      | 5%                        | GOOGL/USD  | Cross    |
| INTC   | 10x      | 10%                       | INTC/USD   | Cross    |
| META   | 20x      | 5%                        | META/USD   | Cross    |
| MSFT   | 20x      | 5%                        | MSFT/USD   | Cross    |
| MU     | 15x      | 6.67%                     | MU/USD     | Cross    |
| NVDA   | 20x      | 5%                        | NVDA/USD   | Isolated |
| SNDK   | 15x      | 6.67%                     | SNDK/USD   | Cross    |
| SPCX   | 15x      | 6.67%                     | SPCX/USD   | Isolated |
| TSLA   | 20x      | 5%                        | TSLA/USD   | Cross    |

<!-- END SOURCE: phoenix/real-world-assets/market-specs.md -->

---

# Source: `sdk/accounts.md`

**Original URL:** `https://docs.phoenix.trade/sdk/accounts.md`

**Local path:** `sdk/accounts.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Accounts

> Manage Phoenix trader account indexes, cross-margin accounts, isolated subaccounts, and trader-state subscriptions with the Rise SDK.

This page covers the trader account model after a trader is eligible to use Phoenix. To activate a trader account, see [Trader Onboarding](/sdk/register).

## Trader account indexes

A Phoenix trader account is a PDA derived from the authority wallet, `pda_index`, `subaccount_index`, and the Phoenix program id. In the TypeScript SDK these are passed as `traderPdaIndex` and `traderSubaccountIndex`; in Rust, `TraderKey::new(...)` uses `(pda_index = 0, subaccount_index = 0)` and `TraderKey::new_with_idx(...)` lets you specify both indexes.

Use `pda_index = 0` for user accounts. With exchange gating enabled, only `pda_index = 0` can be activated; a non-zero PDA index cannot be activated. In the future this will be used to support multiple portfolios.

* `subaccount_index = 0` is the cross-margin account. It is created during onboarding with `max_positions` between `32` and `128` inclusive; the default is `128`.
* `subaccount_index > 0` is an isolated account. Isolated accounts have `max_positions = 1`, so each account is intended for one isolated position.

## Cross vs isolated

Cross margin and isolated margin are represented as separate trader subaccounts under the same authority and `pda_index`.

| Account          | Index                  | Created by                                       | Intended use                                   | State stream behavior                                 |
| ---------------- | ---------------------- | ------------------------------------------------ | ---------------------------------------------- | ----------------------------------------------------- |
| Cross account    | `subaccount_index = 0` | Trader onboarding                                | Shared collateral and many positions           | Included in the trader-state snapshot and deltas      |
| Isolated account | `subaccount_index > 0` | Register child subaccount, then sync from parent | One isolated position with separate collateral | Included in the same trader-state snapshot and deltas |

Subscribe to trader state once per authority and `traderPdaIndex`. The stream returns the cross account and every registered isolated subaccount in the same snapshot, then sends deltas keyed by `subaccountIndex`.

## Trader state across subaccounts

Use the SDK state primitives instead of applying raw snapshot and delta messages yourself. In TypeScript, `createTraderStateStore(client)` exposes `subaccountIndices()`, `subaccount(index)`, `position(index, symbol)`, `orders(index, symbol)`, `triggers(index, symbol)`, and `marginInputs()` across all subaccounts. In Rust, `Trader::apply_update(&msg)` maintains a `Trader` container whose `subaccounts` map includes cross and isolated accounts.

<CodeGroup>
  ```ts TypeScript theme={null}
  import { createPhoenixClient, createTraderStateStore } from "@ellipsis-labs/rise";

  const client = createPhoenixClient({
    apiUrl: "https://perp-api.phoenix.trade",
    rpcUrl: "https://api.mainnet-beta.solana.com",
    ws: { connectMode: "eager" },
    exchangeMetadata: { stream: true },
  });

  const traderState = createTraderStateStore(client);
  const resource = traderState.resource({
    authority: "AUTHORITY_PUBKEY",
    traderPdaIndex: 0,
  });

  const release = resource.retain();
  const snapshot = await resource.ready();

  for (const subaccount of snapshot?.subaccounts ?? []) {
    console.log(subaccount.subaccountIndex, subaccount.collateral);
  }

  const unsubscribe = resource.subscribe((state, previous) => {
    if (state.snapshot === previous.snapshot) return;

    for (const subaccountIndex of resource.subaccountIndices()) {
      const subaccount = resource.subaccount(subaccountIndex);
      console.log({
        subaccountIndex,
        collateral: subaccount?.collateral,
        positions: subaccount?.positionSymbols,
        orderSymbols: subaccount?.orderSymbols,
      });
    }
  });

  // Later, when this component/service no longer needs live state:
  unsubscribe();
  release();
  ```

  ```rust Rust theme={null}
  use phoenix_rise::api::{PhoenixWSClient, Trader, TraderKey};
  use solana_pubkey::Pubkey;

  let authority: Pubkey = "AUTHORITY_PUBKEY".parse()?;
  let key = TraderKey::from_authority(authority);
  let mut trader = Trader::new(key.clone());

  let ws = PhoenixWSClient::new_from_env()?;
  let (mut rx, _handle) = ws.subscribe_to_trader_state(&authority)?;

  while let Some(msg) = rx.recv().await {
      trader.apply_update(&msg);

      for (subaccount_index, subaccount) in &trader.subaccounts {
          println!(
              "subaccount={} collateral={} positions={} orders={}",
              subaccount_index,
              subaccount.collateral,
              subaccount.positions.len(),
              subaccount.orders.len()
          );
      }
  }
  ```
</CodeGroup>

References:

* [TypeScript trader-state store example](https://github.com/Ellipsis-Labs/rise-public/blob/master/ts/examples/04-trader-state-store.ts)
* [Rust trader-state example](https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/sdk/examples/subscribe_trader_state.rs)

## Set position authority

Use the on-chain `DelegateTrader` instruction to set or replace the `position_authority` for a trader account. The current account `authority` signs the transaction; the new position authority is recorded on the trader account but does not need to sign the delegation transaction.

<Tip>
  Set `position_authority` to an embedded wallet, such as a Privy wallet, when you want your app to support one-click transaction signing for trading actions while keeping collateral withdrawals controlled by the user's authority wallet.
</Tip>

The SDK parameter `traderPdaIndex` corresponds to the account's `portfolio_index`, and `traderSubaccountIndex` corresponds to `subaccount_index`.

```ts TypeScript theme={null}
import { createPhoenixClient } from "@ellipsis-labs/rise";

const client = createPhoenixClient({
  apiUrl: "https://perp-api.phoenix.trade",
  rpcUrl: "https://api.mainnet-beta.solana.com",
  exchangeMetadata: { stream: false },
});

const authority = "PHANTOM_WALLET_PUBKEY";
const newPositionAuthority = "PRIVY_WALLET_PUBKEY";

const delegateTraderIx = await client.ixs.buildDelegateTrader({
  traderWallet: authority,
  traderPdaIndex: 0,
  traderSubaccountIndex: 0,
  newPositionAuthority,
});

// Send this instruction in a transaction signed by `authority`.
```

`DelegateTrader` updates one trader account. If you use isolated accounts, each isolated trader account has its own stored `position_authority`. Syncing a child isolated account from its parent cross account copies the parent's current `position_authority` to the child.

### Send market orders with position authority

When a trader account has a separate `position_authority`, build order instructions with the trader account's original `authority`, but make the position authority the signing wallet.

| SDK        | Trader account owner                                 | Market-order signer                                              |
| ---------- | ---------------------------------------------------- | ---------------------------------------------------------------- |
| TypeScript | Pass `authority` as the trader account authority     | Pass `positionAuthority` separately                              |
| Rust       | Derive `TraderKey` from the trader account authority | Pass `position_authority` to `MarketOrderTicket::authority(...)` |

<CodeGroup>
  ```ts TypeScript theme={null}
  import { Side } from "@ellipsis-labs/rise";

  const authority = "PHANTOM_WALLET_PUBKEY";
  const positionAuthority = "PRIVY_WALLET_PUBKEY";
  const symbol = "SOL";

  const marketPacket = await client.orderPackets.buildMarketOrderPacket({
    symbol,
    side: Side.Bid,
    baseUnits: "0.25",
  });

  const placeMarketIx = await client.ixs.placeMarketOrder({
    authority,
    positionAuthority,
    symbol,
    orderPacket: marketPacket,
  });

  // Send the transaction with `positionAuthority` as the signer.
  ```

  ```rust Rust theme={null}
  use phoenix_rise::{MarketOrderTicket, PhoenixTxBuilder, Side, TraderKey};

  let authority = phantom_wallet_pubkey;
  let position_authority = privy_wallet_pubkey;
  let trader = TraderKey::new(authority);
  let builder = PhoenixTxBuilder::new(&metadata);

  let market_ixs = builder
      .place_market_order(
          MarketOrderTicket::builder()
              .authority(position_authority)
              .trader_account(trader.pda())
              .symbol("SOL")
              .side(Side::Bid)
              .num_base_lots(25_000)
              .build()?,
      )
      .await?;

  // Send the transaction with `position_authority` as the signer.
  ```
</CodeGroup>

In TypeScript, `authority` is still used to resolve the trader PDA, and `positionAuthority` becomes the signer account in the Phoenix instruction. In Rust, `TraderKey::new(authority)` still resolves the trader PDA from the original wallet authority, but the ticket's `.authority(position_authority)` is the signer placed into the market-order instruction.

<Note>
  Do not derive a new trader account from the position authority. The trader account remains the PDA for the user's authority wallet; only the signer changes.
</Note>

## Create isolated accounts

Onboarding creates the user's cross-margin trader account at `traderPdaIndex = 0` and `traderSubaccountIndex = 0`. After that cross account exists, create isolated accounts by registering a non-zero subaccount index under `traderPdaIndex = 0`, then syncing the parent cross account to the child isolated account.

The sync instruction copies the parent account's current capabilities and fee configuration to the isolated account.

<CodeGroup>
  ```ts TypeScript theme={null}
  import { MarginType, createPhoenixClient } from "@ellipsis-labs/rise";

  const client = createPhoenixClient({
    apiUrl: "https://perp-api.phoenix.trade",
    rpcUrl: "https://api.mainnet-beta.solana.com",
    exchangeMetadata: { stream: false },
  });

  const authority = "AUTHORITY_PUBKEY";
  const traderPdaIndex = 0;
  const isolatedSubaccountIndex = 1;

  const registerIsolatedIx = await client.ixs.buildRegisterTrader({
    authority,
    marginType: MarginType.Isolated,
    traderPdaIndex,
    traderSubaccountIndex: isolatedSubaccountIndex,
  });

  const syncIsolatedIx = await client.ixs.buildSyncParentToChild({
    traderWallet: authority,
    traderPdaIndex,
    traderSubaccountIndex: isolatedSubaccountIndex,
  });

  const instructions = [registerIsolatedIx, syncIsolatedIx];

  // Send the instructions in a transaction signed by `authority`.
  // The parent cross account must already exist.
  ```

  ```rust Rust theme={null}
  use phoenix_rise::{PhoenixHttpClient, PhoenixMetadata, PhoenixTxBuilder, TraderKey};
  use solana_keypair::read_keypair_file;
  use solana_signer::Signer;

  let keypair = read_keypair_file("PATH_TO_KEYPAIR.json")?;
  let parent_key = TraderKey::new(keypair.pubkey()); // pda_index 0, subaccount_index 0
  let child_key = TraderKey::new_with_idx(parent_key.authority(), 0, 1);

  let http = PhoenixHttpClient::new_from_env()?;
  let exchange = http.get_exchange().await?.into();
  let metadata = PhoenixMetadata::new(exchange);
  let builder = PhoenixTxBuilder::new(&metadata);

  let mut instructions = builder.build_register_trader(
      child_key.authority(),
      child_key.pda_index,
      child_key.subaccount_index,
  )?;
  instructions.extend(builder.build_sync_parent_to_child(
      child_key.authority(),
      parent_key.pda(),
      child_key.pda(),
  )?);

  // Send the instructions in a transaction signed by `authority`.
  // The parent cross account must already exist.
  ```
</CodeGroup>

<Note>
  Use `traderSubaccountIndex = 0` only for the cross account. Isolated accounts use non-zero subaccount indexes.
</Note>

## Fund isolated accounts and open positions

When you open an isolated position yourself, mirror the same instruction order used by the server-built isolated order routes:

1. Pick a non-zero isolated subaccount index.
2. If that child account does not exist, register it.
3. Sync the parent cross account to the child account.
4. Transfer collateral from the parent cross account to the child account with `TransferCollateral`.
5. Place the order on the child account.
6. Optionally append `TransferCollateralChildToParent` to sweep child collateral back to the parent when the child has no active limit orders or positions, including when the order you just placed closes the isolated position.

<Note>
  The parent cross account is subaccount `0`. `TransferCollateral` moves a specific amount between two trader accounts; use `srcSubaccountIndex = 0` and `dstSubaccountIndex = childSubaccountIndex` to fund an isolated child. TypeScript amounts are native USDC units, so `1 USDC = 1_000_000n`. The Rust builder accepts decimal USDC amounts.
</Note>

<Note>
  Unused isolated collateral does not stay parked forever. The server-built isolated order routes append `TransferCollateralChildToParent` by default, which sweeps collateral back to cross margin in the same transaction if the order closes the position and leaves no active limit orders. An off-chain crank can also sweep idle isolated collateral when the subaccount has no active limit orders or positions.
</Note>

<CodeGroup>
  ```ts TypeScript theme={null}
  import {
    MarginType,
    OrderFlags,
    SelfTradeBehavior,
    Side,
    createPhoenixClient,
  } from "@ellipsis-labs/rise";

  const client = createPhoenixClient({
    apiUrl: "https://perp-api.phoenix.trade",
    rpcUrl: "https://api.mainnet-beta.solana.com",
    exchangeMetadata: { stream: false },
  });

  const authority = "AUTHORITY_PUBKEY";
  const traderPdaIndex = 0;
  const childSubaccountIndex = 1;
  const symbol = "SOL";
  const usdc = (wholeUsdc: bigint) => wholeUsdc * 1_000_000n;

  const traderSnapshot = await client.api
    .traders()
    .getTraderStateSnapshot(authority, { traderPdaIndex });

  const childExists = traderSnapshot.snapshot.subaccounts.some(
    (subaccount) => subaccount.subaccountIndex === childSubaccountIndex
  );

  const instructions = [];

  if (!childExists) {
    instructions.push(
      await client.ixs.buildRegisterTrader({
        authority,
        marginType: MarginType.Isolated,
        traderPdaIndex,
        traderSubaccountIndex: childSubaccountIndex,
      })
    );
  }

  instructions.push(
    await client.ixs.buildSyncParentToChild({
      traderWallet: authority,
      traderPdaIndex,
      traderSubaccountIndex: childSubaccountIndex,
    })
  );

  instructions.push(
    await client.ixs.buildTransferCollateral({
      authority,
      traderPdaIndex,
      srcSubaccountIndex: 0,
      dstSubaccountIndex: childSubaccountIndex,
      amount: usdc(100n),
    })
  );

  const orderPacket = await client.ixs.orderPackets.buildLimitOrderPacket({
    symbol,
    side: Side.Bid,
    priceUsd: "150",
    baseUnits: "1",
    selfTradeBehavior: SelfTradeBehavior.CancelProvide,
    orderFlags: OrderFlags.None,
    cancelExisting: false,
  });

  instructions.push(
    await client.ixs.buildPlaceLimitOrder({
      authority,
      symbol,
      orderPacket,
      traderPdaIndex,
      traderSubaccountIndex: childSubaccountIndex,
    })
  );

  // Optional: mimics the server isolated-order routes' default cleanup step.
  // If this order closes the isolated position and leaves no resting orders,
  // the remaining child collateral is moved back to the cross account.
  instructions.push(
    await client.ixs.buildTransferCollateralChildToParent({
      authority,
      traderPdaIndex,
      childSubaccountIndex,
    })
  );

  // Send the instructions in a transaction signed by `authority`.
  // If you use `positionAuthority`, pass it to transfer/order builders and
  // sign the relevant instructions with that delegated authority.
  ```

  ```rust Rust theme={null}
  use phoenix_rise::{
      LimitOrderTicket, PhoenixHttpClient, PhoenixMetadata, PhoenixTxBuilder, Side, TraderKey,
  };
  use solana_keypair::read_keypair_file;
  use solana_signer::Signer;

  let keypair = read_keypair_file("PATH_TO_KEYPAIR.json")?;
  let parent_key = TraderKey::new(keypair.pubkey()); // pda_index 0, subaccount_index 0
  let child_key = TraderKey::new_with_idx(parent_key.authority(), 0, 1);

  let http = PhoenixHttpClient::new_from_env()?;
  let exchange = http.get_exchange().await?.into();
  let metadata = PhoenixMetadata::new(exchange);
  let builder = PhoenixTxBuilder::new(&metadata);

  let child_exists = http
      .traders()
      .get_trader_subaccount(
          &parent_key.authority(),
          child_key.pda_index,
          child_key.subaccount_index,
      )
      .await?
      .is_some();

  let mut instructions = Vec::new();

  if !child_exists {
      instructions.extend(builder.build_register_trader(
          child_key.authority(),
          child_key.pda_index,
          child_key.subaccount_index,
      )?);
  }

  instructions.extend(builder.build_sync_parent_to_child(
      child_key.authority(),
      parent_key.pda(),
      child_key.pda(),
  )?);

  instructions.extend(builder.build_transfer_collateral(
      child_key.authority(),
      parent_key.pda(),
      child_key.pda(),
      100.0,
  )?);

  let order_ticket = LimitOrderTicket::builder()
      .authority(child_key.authority())
      .trader_account(child_key.pda())
      .symbol("SOL")
      .side(Side::Bid)
      .price(150.0)
      .num_base_lots(1)
      .subaccount_index(child_key.subaccount_index)
      .build()?;

  instructions.extend(builder.place_limit_order(order_ticket).await?);

  // Optional: mimics the server isolated-order routes' default cleanup step.
  // If this order closes the isolated position and leaves no resting orders,
  // the remaining child collateral is moved back to the cross account.
  instructions.extend(builder.build_transfer_collateral_child_to_parent(
      child_key.authority(),
      child_key.pda(),
      parent_key.pda(),
  )?);

  // Send the instructions in a transaction signed by `authority`.
  ```
</CodeGroup>

The server routes also check that the parent has enough transferable collateral before creating the transfer instruction. If your app builds these instructions client-side, perform the same check from trader state so you do not try to move collateral reserved for existing margin requirements.

## Return isolated collateral to cross margin

Use `TransferCollateralChildToParent` when an isolated child account should return its collateral to the parent cross account. This instruction targets one child subaccount and transfers all available collateral back to subaccount `0`; it does not take an amount parameter.

The child account must have no active limit orders or positions. If the child account has no collateral left to transfer, the instruction no-ops on-chain.

This is useful after a close-position order. Place the close order first, then append `TransferCollateralChildToParent`; if the close leaves the isolated account with no active position and no active limit orders, the remaining collateral is moved back to the cross account.

<CodeGroup>
  ```ts TypeScript theme={null}
  const authority = "AUTHORITY_PUBKEY";
  const traderPdaIndex = 0;
  const isolatedSubaccountIndex = 1;

  const transferToParentIx =
    await client.ixs.buildTransferCollateralChildToParent({
      authority,
      traderPdaIndex,
      childSubaccountIndex: isolatedSubaccountIndex,
    });

  // Send the instruction in a transaction signed by `authority`.
  // The parent cross account and child isolated account must already exist.
  ```

  ```rust Rust theme={null}
  let parent_key = TraderKey::new(keypair.pubkey()); // pda_index 0, subaccount_index 0
  let child_key = TraderKey::new_with_idx(parent_key.authority(), 0, 1);

  let transfer_ixs = builder.build_transfer_collateral_child_to_parent(
      child_key.authority(),
      child_key.pda(),
      parent_key.pda(),
  )?;

  // Send the instructions in a transaction signed by `authority`.
  // The parent cross account and child isolated account must already exist.
  ```
</CodeGroup>

<!-- END SOURCE: sdk/accounts.md -->

---

# Source: `sdk/auth.md`

**Original URL:** `https://docs.phoenix.trade/sdk/auth.md`

**Local path:** `sdk/auth.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Auth

> JWT lifecycle for authenticated Phoenix API and Rise SDK sessions.

Phoenix auth issues short-lived access JWTs and longer-lived refresh tokens. The Rise SDK can manage the session, attach bearer tokens to authenticated requests, refresh before expiry, and persist the rotated token pair.

Most Phoenix market and trader read routes are public. You can fetch exchange params, market stats, orderbooks, candles, and trader state without first creating a session.

Use auth when your integration needs an identified caller, higher API limits for authenticated traffic, or authenticated workflows. Auth is required for referral-code onboarding with `/v1/referral/activate-tx` and for notifications.

<Note>
  Auth does not replace Solana transaction signing. Phoenix instructions still need the correct wallet, authority, position authority, or fee payer signature depending on the instruction.
</Note>

## Lifecycle

1. Request a challenge from Phoenix.
2. Sign the challenge with the authority wallet.
3. Exchange the signature for an auth response.
4. Use `Authorization: Bearer <access_token>` on authenticated routes.
5. Refresh with `POST /v1/auth/refresh` before the access token expires.
6. Reauthenticate when refresh fails with a terminal auth error.

The auth response shape is:

```json theme={null}
{
  "token_type": "Bearer",
  "access_token": "eyJ...",
  "expires_in": 900,
  "refresh_token": "eyJ...",
  "refresh_expires_in": 2592000
}
```

Store the full response. Refresh rotates the access token and refresh token; the previous access token is accepted for a short grace window.

## Endpoints

| Step                         | Endpoint                                     | Body or query                                     | Notes                                                                               |
| ---------------------------- | -------------------------------------------- | ------------------------------------------------- | ----------------------------------------------------------------------------------- |
| Wallet nonce                 | `GET /v1/auth/nonce?wallet_pubkey=...`       | query `wallet_pubkey`                             | Returns `nonce_id`, `message`, and `expires_at`.                                    |
| Wallet login                 | `POST /v1/auth/login/wallet`                 | `wallet_pubkey`, `signature`, `nonce_id`          | `signature` signs the exact nonce `message`.                                        |
| Wallet transaction challenge | `POST /v1/auth/wallet/transaction-challenge` | `wallet_pubkey`                                   | Alternative for wallets that cannot sign arbitrary messages.                        |
| Wallet transaction login     | `POST /v1/auth/login/wallet/transaction`     | `wallet_pubkey`, `nonce_id`, `signed_transaction` | Exchanges the signed memo transaction for JWTs.                                     |
| Refresh                      | `POST /v1/auth/refresh`                      | `refresh_token`                                   | SDKs include the current bearer token when available and store the rotated session. |
| Logout                       | `POST /v1/auth/logout`                       | none                                              | Requires `Authorization: Bearer <access_token>` and revokes the session.            |

Routes that can benefit from or require auth:

* Referral-code onboarding: see [Trader Onboarding](/sdk/register#with-a-referral-code).
* Notifications over REST or WebSocket.
* Any protected account, user, or builder workflow marked with bearer auth in the [API reference](/api).
* Authenticated integrations that need higher rate limits than anonymous public traffic.

## Wallet login

This TypeScript example is written for a browser wallet that supports `signMessage`.

<CodeGroup>
  ```ts TypeScript theme={null}
  import {
    LocalStorageAuthSessionStorage,
    createPhoenixClient,
  } from "@ellipsis-labs/rise";
  import bs58 from "bs58";

  const client = createPhoenixClient({
    apiUrl: "https://perp-api.phoenix.trade",
    auth: true,
    authConfig: {
      storage: new LocalStorageAuthSessionStorage(),
    },
    ws: false,
  });

  const auth = client.auth!;
  const walletPubkey = wallet.publicKey.toString();

  const nonce = await auth.getWalletNonce(walletPubkey);
  const message = new TextEncoder().encode(nonce.message);
  const signature = await wallet.signMessage(message);

  const session = await auth.loginWithWalletSignature(
    walletPubkey,
    bs58.encode(signature),
    nonce.nonce_id
  );

  console.log(session.accessToken);
  ```

  ```rust Rust theme={null}
  use phoenix_rise::PhoenixHttpClient;
  use solana_keypair::Keypair;
  use solana_signer::Signer;

  let client = PhoenixHttpClient::builder("https://perp-api.phoenix.trade")
      .enable_auth()
      .build()?;
  let auth = client.auth().expect("auth enabled");

  let wallet = Keypair::new(); // Load the user's wallet in production.
  let wallet_pubkey = wallet.pubkey().to_string();

  let nonce = auth.get_wallet_nonce(&wallet_pubkey).await?;
  let signature = wallet.sign_message(nonce.message.as_bytes());

  let session = auth
      .login_with_wallet_signature(
          &wallet_pubkey,
          signature.as_ref(),
          nonce.nonce_id,
      )
      .await?;

  println!("{}", session.access_token());
  ```
</CodeGroup>

<Tip>
  In Rust builds with the `solana-keypair` feature, `auth.login_with_wallet_keypair(&keypair).await?` wraps the nonce, signing, and login sequence.
</Tip>

## Refresh

With a managed SDK session, refresh is normally automatic before authenticated HTTP and WebSocket requests. Manual refresh is still useful when you want to force rotation after loading a saved session.

<CodeGroup>
  ```ts TypeScript theme={null}
  const auth = client.auth!;
  const manager = client.sessionManager!;
  const current = await manager.getSession();

  if (!current) {
    throw new Error("No Phoenix auth session");
  }

  const refreshed = await auth.refresh(current.refreshToken);
  console.log(refreshed.accessToken);
  ```

  ```rust Rust theme={null}
  let auth = client.auth().expect("auth enabled");
  let refreshed = auth.refresh_session().await?;

  println!("{}", refreshed.access_token());
  ```
</CodeGroup>

Terminal refresh failures mean the user must sign in again:

* `invalid_refresh_token`
* `refresh_expired`
* `session_missing`

## Auth errors

Auth errors use the standard API error shape:

```json theme={null}
{
  "error": "invalid_access_token"
}
```

Common unauthenticated or reauthentication cases:

| Error code              | Status         | Meaning                                                                                           |
| ----------------------- | -------------- | ------------------------------------------------------------------------------------------------- |
| `missing_access_token`  | `401`          | The route requires an authenticated access token, but none was available to the route guard.      |
| `missing_bearer_token`  | `401`          | The route expects an `Authorization: Bearer ...` header.                                          |
| `invalid_access_token`  | `401`          | The access JWT is malformed, expired, signed by an unknown key, or otherwise failed verification. |
| `access_token_expired`  | `401`          | The access token is expired. Refresh and retry.                                                   |
| `session_missing`       | `401` or `404` | The server-side session was revoked, expired, or missing. Reauthenticate.                         |
| `access_jti_mismatch`   | `401`          | The access token is no longer the current token for the session. Refresh or reauthenticate.       |
| `invalid_refresh_token` | `401`          | The refresh token is invalid, expired, or already consumed. Reauthenticate.                       |
| `refresh_expired`       | SDK-side       | The SDK knows the stored refresh token is past its expiry. Reauthenticate.                        |
| `no_auth_session`       | SDK-side       | Auth was enabled, but no session is loaded. Sign in first.                                        |
| `user_only`             | `403`          | The route requires user auth.                                                                     |
| `admin_only`            | `403`          | The route requires admin auth.                                                                    |
| `insufficient_role`     | `403`          | The authenticated role cannot access the route.                                                   |

<!-- END SOURCE: sdk/auth.md -->

---

# Source: `sdk/collateral.md`

**Original URL:** `https://docs.phoenix.trade/sdk/collateral.md`

**Local path:** `sdk/collateral.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Collateral

> Build Phoenix USDC deposit, withdrawal, and collateral verification flows with the TypeScript and Rust SDKs.

Phoenix perps are margined in USDC. The SDK deposit and withdrawal helpers build Solana instructions; your app still signs and sends the transaction with the trader authority wallet.

Deposit and withdrawal amounts are expressed in native USDC units when using the TypeScript SDK: `1 USDC = 1_000_000n`. The Rust `PhoenixTxBuilder` convenience methods accept decimal USDC amounts as `f64`.

<Note>
  Deposit and withdrawal flows target a Phoenix trader account. Register the trader account first if it does not already exist. Withdrawals are subject to Phoenix account health checks and the protocol withdraw queue; see [Collateral](/phoenix/collateral-and-accounts/collateral) for more detail.
</Note>

## TypeScript

Use `createPhoenixClient(...)` and the `client.ixs` helpers when you want the SDK to resolve exchange accounts, Ember accounts, trader PDAs, and token accounts for you.

`buildDepositIxs(...)` returns three instructions:

1. Create the Phoenix token ATA if needed.
2. Convert wallet USDC to Phoenix canonical tokens through Ember.
3. Deposit the Phoenix tokens into the Phoenix trader account.

`buildWithdrawIxs(...)` returns five instructions:

1. Create the Phoenix token ATA if needed.
2. Approve Ember to spend Phoenix canonical tokens.
3. Create the USDC ATA if needed.
4. Withdraw Phoenix tokens from the Phoenix trader account.
5. Convert Phoenix tokens back to wallet USDC through Ember.

```ts TypeScript theme={null}
import { createPhoenixClient } from "@ellipsis-labs/rise";

const client = createPhoenixClient({
  apiUrl: "https://perp-api.phoenix.trade",
  rpcUrl: "https://api.mainnet-beta.solana.com",
  exchangeMetadata: { stream: false },
});

const authority = "AUTHORITY_PUBKEY";
const traderPdaIndex = 0;
const traderSubaccountIndex = 0;
const usdc = (wholeUsdc: bigint) => wholeUsdc * 1_000_000n;

const deposit = await client.ixs.buildDepositIxs({
  authority,
  traderPdaIndex,
  traderSubaccountIndex,
  amount: usdc(100n),
});

const withdraw = await client.ixs.buildWithdrawIxs({
  authority,
  traderPdaIndex,
  traderSubaccountIndex,
  amount: usdc(25n),
});

console.log(deposit.instructions);
console.log(withdraw.instructions);

// Send each instruction array in a transaction signed by `authority`.
// Wallet and transaction submission code depends on your Solana client stack.
```

You can also build the protocol instructions individually with `buildEmberDeposit`, `buildDepositFunds`, `buildWithdrawFunds`, and `buildEmberWithdraw`, but most integrations should prefer the full flow helpers above.

## Rust

Use `PhoenixHttpClient` to fetch exchange metadata, then pass that metadata into `PhoenixTxBuilder`. The builder returns instruction arrays that should be sent in a single transaction signed by the trader authority.

```rust Rust theme={null}
use phoenix_rise::{PhoenixHttpClient, PhoenixMetadata, PhoenixTxBuilder, TraderKey};
use solana_commitment_config::CommitmentConfig;
use solana_keypair::read_keypair_file;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_signer::Signer;
use solana_transaction::Transaction;

const RPC_ENDPOINT: &str = "https://api.mainnet-beta.solana.com";

let keypair = read_keypair_file("PATH_TO_KEYPAIR.json")?;
let trader_key = TraderKey::new(keypair.pubkey());

let http = PhoenixHttpClient::new_from_env()?;
let exchange = http.get_exchange().await?.into();
let metadata = PhoenixMetadata::new(exchange);
let builder = PhoenixTxBuilder::new(&metadata);

let rpc = RpcClient::new_with_commitment(
    RPC_ENDPOINT.to_string(),
    CommitmentConfig::confirmed(),
);

let deposit_ixs = builder.build_deposit_funds(
    trader_key.authority(),
    trader_key.pda(),
    100.0,
)?;
let deposit_blockhash = rpc.get_latest_blockhash().await?;
let deposit_tx = Transaction::new_signed_with_payer(
    &deposit_ixs,
    Some(&trader_key.authority()),
    &[&keypair],
    deposit_blockhash,
);
let deposit_signature = rpc.send_and_confirm_transaction(&deposit_tx).await?;

let withdraw_ixs = builder.build_withdraw_funds(
    trader_key.authority(),
    trader_key.pda(),
    25.0,
)?;
let withdraw_blockhash = rpc.get_latest_blockhash().await?;
let withdraw_tx = Transaction::new_signed_with_payer(
    &withdraw_ixs,
    Some(&trader_key.authority()),
    &[&keypair],
    withdraw_blockhash,
);
let withdraw_signature = rpc.send_and_confirm_transaction(&withdraw_tx).await?;

println!("deposit: {deposit_signature}");
println!("withdraw: {withdraw_signature}");
```

For a complete runnable Rust example, see [`rust/sdk/examples/deposit_funds.rs`](https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/sdk/examples/deposit_funds.rs).

## Monitoring

After a transaction confirms, fetch trader state or collateral history to verify the account:

<CodeGroup>
  ```ts TypeScript theme={null}
  const trader = await client.api
    .traders()
    .getTraderStateSnapshot(authority, { traderPdaIndex });

  const subaccount = trader.snapshot.subaccounts.find(
    (subaccount) => subaccount.subaccountIndex === traderSubaccountIndex
  );
  console.log("collateral:", subaccount?.collateral ?? "0");

  const history = await client.api
    .collateral()
    .getTraderCollateralHistory(authority, {
      pdaIndex: traderPdaIndex,
      limit: 10,
    });
  ```

  ```rust Rust theme={null}
  use phoenix_rise::CollateralHistoryQueryParams;

  let traders = http.traders().get_trader(&trader_key.authority()).await?;
  if let Some(trader) = traders
      .iter()
      .find(|trader| trader.trader_subaccount_index == trader_key.subaccount_index)
  {
      println!("collateral: {}", trader.collateral_balance.ui);
  }

  let history = http
      .collateral()
      .get_trader_collateral_history(
          &trader_key,
          CollateralHistoryQueryParams::new(10),
      )
      .await?;
  ```
</CodeGroup>

<!-- END SOURCE: sdk/collateral.md -->

---

# Source: `sdk/litesvm-testing.md`

**Original URL:** `https://docs.phoenix.trade/sdk/litesvm-testing.md`

**Local path:** `sdk/litesvm-testing.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# LiteSVM Testing

> Use phoenix-rise-litesvm-test fixtures to test Phoenix program integrations locally.

`phoenix-rise-litesvm-test` provides deterministic LiteSVM fixtures for Phoenix program integration tests. It loads Phoenix Eternal and Ember, optionally Hawkeye and Flight, creates fake USDC collateral, initializes markets, seeds actors, and exposes helpers for replaying setup and action transactions.

References:

* [phoenix-rise-litesvm-test README](https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/litesvm-test/README.md)
* [LiteSVM fixture crate](https://github.com/Ellipsis-Labs/rise-public/tree/master/rust/litesvm-test)
* [SDK localnet fixture tests](https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/sdk/tests/sdk_localnet_fixture_tests.rs)
* [Example program tests](https://github.com/Ellipsis-Labs/rise-public/tree/master/programs/example-program/tests)
* [TypeScript LiteSVM harness](https://github.com/Ellipsis-Labs/rise-public/tree/master/ts/tests/test-harness)
* [TypeScript SDK localnet VM tests](https://github.com/Ellipsis-Labs/rise-public/blob/master/ts/tests/sdk-localnet-vm.test.ts)
* [TypeScript SDK localnet flow tests](https://github.com/Ellipsis-Labs/rise-public/blob/master/ts/tests/sdk-localnet-flows.test.ts)

## Setup

The fixture helpers look for local protocol SBF artifacts. In CI you can use the fixture package directly; locally, set the artifact paths or point the helpers at a Phoenix repo checkout.

```bash theme={null}
export PHOENIX_REPO_ROOT=/path/to/phoenix
export RISE_SDK_LOCALNET_REQUIRE_PROGRAMS=1
```

For the example program tests, also build or point to the example program SBF:

```bash theme={null}
export RISE_EXAMPLE_PROGRAM_SO=/path/to/rise_example_program.so
```

## Context

Build a `SdkLocalnetContext`, load any extra program under test, execute fixture setup, then send instructions through the context.

```rust Rust theme={null}
use phoenix_rise_litesvm_test::{
    SdkLocalnetContext, SdkLocalnetProgram, default_sdk_localnet_fixture,
    find_sdk_localnet_program_paths,
};
use solana_pubkey::Pubkey;

let fixture = default_sdk_localnet_fixture()?;
let program_paths = find_sdk_localnet_program_paths()
    .expect("Phoenix protocol SBF artifacts are required");
let program_id = Pubkey::new_unique();

let mut context = SdkLocalnetContext::new_with_programs(
    fixture,
    program_paths,
    [SdkLocalnetProgram::new(program_id, "target/deploy/my_program.so")],
);

context.execute_setup();
context.svm.warp_to_slot(200);
```

Source: [litesvm-test/src/context.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/litesvm-test/src/context.rs).

## Prime Markets

Most order tests need current oracle and spline prices plus maker liquidity. The example program primes those from fixture transactions before sending taker instructions.

```rust Rust theme={null}
pub fn prime_market(context: &mut SdkLocalnetContext) {
    context.send_fixture_transaction("oracleSetPrices");
    context.send_fixture_transaction("splineUpdatePrices");
    context.send_fixture_transaction("orderbookPlaceLevels");
}
```

Source: [example-program/tests/common/mod.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/tests/common/mod.rs).

## Test A Flow

Write tests against the behavior you expect from your program, not against Phoenix internals. This example places a market order, places a limit order, extracts the order id from return data/log helpers, and cancels it.

```rust Rust theme={null}
#[test]
fn market_limit_and_cancel_flow_executes() {
    let Some((mut context, program_id)) = setup_context() else {
        return;
    };
    prime_market(&mut context);

    let actor = context.actor("taker0");
    let market = context.market("BTC");

    let market_logs = send_and_print_logs(
        &mut context,
        with_compute_budget(place_market_ix(
            &context,
            &actor,
            &market,
            program_id,
            Side::Bid,
            BTC_BASE_LOTS,
            quote_lots_for_price(&market, 100_500.0, 6, BTC_BASE_LOTS),
            11,
            OrderFlags::None,
        )),
        &actor.seed,
        "place-market-order",
    );
    assert_logs_contain(&market_logs, "Phoenix place market order");
}
```

Source: [example-program/tests/market\_orders.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/tests/market_orders.rs).

## TypeScript LiteSVM

The TypeScript SDK tests use the `litesvm` npm package with the same generated
localnet fixture. The published package exposes the fixture at
`@ellipsis-labs/rise/test-fixtures/default-localnet.json`; the public repo's
test harness shows how to load local SBF artifacts, replay setup transactions,
attach fixture signers, and send SDK-built instructions into LiteSVM.

```ts TypeScript theme={null}
import { LiteSVM } from "litesvm";
import defaultFixture from "@ellipsis-labs/rise/test-fixtures/default-localnet.json";

const vm = new LiteSVM();

// Load local Phoenix/Ember/Hawkeye SBF artifacts, fund fixture signers,
// then replay defaultFixture.setupTransactions before sending test actions.
```

Source: [ts/tests/test-harness/localnet.ts](https://github.com/Ellipsis-Labs/rise-public/blob/master/ts/tests/test-harness/localnet.ts).

The public harness wraps those steps in `createSdkLocalnetContext` and exposes
helpers for fixture-backed market params, account contexts, signing, and
transaction submission.

```ts TypeScript theme={null}
import {
  buildMarketOrderPacketFromMarketParams,
  buildPlaceMarketOrderIxResolved,
  Side,
} from "@ellipsis-labs/rise";
import {
  buildSdkLocalnetMarketParams,
  buildSdkLocalnetPlaceOrderContext,
  createSdkLocalnetContext,
} from "./test-harness/litesvm";

const context = await createSdkLocalnetContext();
const actor = context.getActor("taker0");
const orderPacket = buildMarketOrderPacketFromMarketParams(
  {
    side: Side.Bid,
    priceLimitUsd: "101000",
    baseUnits: "0.01",
    minBaseUnitsToFill: "0.01",
  },
  buildSdkLocalnetMarketParams(context, "BTC")
);

const ix = buildPlaceMarketOrderIxResolved({
  ...buildSdkLocalnetPlaceOrderContext(context, {
    actorName: actor.name,
    symbol: "BTC",
  }),
  orderPacket,
});

await context.sendInstructions([ix], {
  feePayerSeed: actor.seed,
  label: "sdk-built-place-market-order",
});
```

Source: [ts/tests/sdk-localnet-vm.test.ts](https://github.com/Ellipsis-Labs/rise-public/blob/master/ts/tests/sdk-localnet-vm.test.ts).

The broader TypeScript flow tests cover the same localnet surface as the Rust
fixture tests: deposits, withdrawals, market orders, limit orders, stop losses,
conditionals, Hawkeye return data, and isolated subaccounts.

## Fixture Metadata

When your off-chain test client needs the same market params as LiteSVM, build `PhoenixMetadata` from the fixture instead of calling mainnet APIs.

```rust Rust theme={null}
use phoenix_rise_litesvm_test::{
    default_sdk_localnet_fixture, phoenix_metadata_from_fixture,
};

let fixture = default_sdk_localnet_fixture()?;
let metadata = phoenix_metadata_from_fixture(&fixture)?;
let builder = PhoenixTxBuilder::new(&metadata);
```

Use this pattern to test `PhoenixTxBuilder` flows against the local fixture and keep transaction construction deterministic.

<!-- END SOURCE: sdk/litesvm-testing.md -->

---

# Source: `sdk/margin.md`

**Original URL:** `https://docs.phoenix.trade/sdk/margin.md`

**Local path:** `sdk/margin.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Margin

> Compute Phoenix trader margin and liquidation estimates with the Rise SDK.

Use this page when you need account health, margin requirements, transferable collateral, or liquidation estimates after you have live trader state. For account setup and trader-state subscriptions across subaccounts, see [Accounts](/sdk/accounts).

## Join Market Prices

For margin, liquidation, and account-health views, subscribe to trader state and market stats for the symbols with open positions or orders. Let trader-state resources maintain positions/orders and market-data resources maintain mark prices.

<CodeGroup>
  ```ts TypeScript theme={null}
  const marketData = client.marketData();
  const releaseMarketData = marketData.retain();
  await marketData.ready();

  resource.subscribe(() => {
    const symbols = new Set<string>();

    for (const subaccountIndex of resource.subaccountIndices()) {
      const subaccount = resource.subaccount(subaccountIndex);
      for (const symbol of subaccount?.positionSymbols ?? []) {
        symbols.add(symbol);
      }
      for (const symbol of subaccount?.orderSymbols ?? []) {
        symbols.add(symbol);
      }
    }

    for (const symbol of symbols) {
      const row = marketData.market(symbol);
      console.log(symbol, row?.markPrice);
    }
  });
  ```

  ```rust Rust theme={null}
  use phoenix_rise::api::{PhoenixHttpClient, PhoenixMetadata, PhoenixWSClient};

  let http = PhoenixHttpClient::new_from_env()?;
  let exchange = http.get_exchange().await?.into();
  let mut metadata = PhoenixMetadata::new(exchange);

  let ws = PhoenixWSClient::new_from_env()?;
  let (mut market_rx, _handle) = ws.subscribe_to_market("SOL".to_string())?;

  while let Some(stats) = market_rx.recv().await {
      metadata.apply_market_stats(&stats)?;
      println!("{} mark={}", stats.symbol, stats.mark_price);
  }
  ```
</CodeGroup>

## Local Margin

Use trader-state `marginInputs()` and market params to compute local margin. For real-time monitoring, update market prices from WebSocket streams before recomputing.

<CodeGroup>
  ```ts TypeScript theme={null}
  import {
    MarginMarketParamsStore,
    createMarginCalculator,
  } from "@ellipsis-labs/rise";

  const paramsStore = new MarginMarketParamsStore({
    client: client.api,
    autoRefreshIntervalMs: 30_000,
  });

  const paramsBySymbol = await paramsStore.getMarketParamsBySymbol();
  const calculator = createMarginCalculator(Object.values(paramsBySymbol));
  const marginInputs = resource.marginInputs();

  if (marginInputs) {
    const margin = calculator.computeTraderMarginFromInputs(marginInputs);
    console.log(margin.subaccounts[0]?.margin);
  }
  ```

  ```rust Rust theme={null}
  if let Some(cross) = trader.primary_subaccount() {
      let portfolio = cross.to_trader_portfolio();
      let margin = portfolio.compute_margin(metadata.all_perp_asset_metadata())?;

      println!(
          "maintenance margin={}",
          margin.margin.maintenance_margin.as_inner()
      );
  }
  ```
</CodeGroup>

Reference: [Rust live margin example](https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/sdk/examples/compute_trader_margin.rs).

## Cache-Backed Liquidation Estimates

For fast UI updates and risk monitoring, estimate liquidation prices from local trader state plus the exchange cache. This avoids API polling and uses the same SDK margin primitives described above.

The examples below binary-search the mark price where the subaccount risk tier crosses into `liquidatable` or worse. This keeps the estimate tied to SDK margin math, leverage tiers, funding, discounted uPnL, and limit-order margin. Treat it as a local estimate: stream freshness and search bounds matter. Use Hawkeye for the authoritative on-chain simulation.

In TypeScript, the flow is `client.exchange.ready()`, `client.exchange.market(symbol)`, `client.marketData().market(symbol)`, `resource.marginInputs()`, then `createMarginCalculator(...)`. In Rust, the flow is `PhoenixMetadata::apply_market_stats(&stats)`, `Trader::apply_update(&msg)`, `SubaccountState::to_trader_portfolio()`, then `TraderPortfolio::compute_margin(metadata.all_perp_asset_metadata())`.

<CodeGroup>
  ```ts TypeScript theme={null}
  import {
    createMarginCalculator,
    priceUsdToTicksWithMarketParams,
    type ExchangeMarketSnapshot,
    type MarketParams,
    type SubaccountMarginInputs,
  } from "@ellipsis-labs/rise";

  const bps = (value: number | undefined, fallback: number) =>
    String(value ?? fallback);

  const paramsFromExchangeCache = (
    market: ExchangeMarketSnapshot,
    markPriceUsd: number
  ): MarketParams => ({
    symbol: market.symbol,
    assetId: market.assetId,
    markPriceTicks: priceUsdToTicksWithMarketParams(markPriceUsd, {
      tickSize: market.tickSize,
      baseLotsDecimals: market.baseLotsDecimals,
    }).toString(),
    tickSize: market.tickSize.toString(),
    baseLotDecimals: market.baseLotsDecimals,
    leverageTiers: market.leverageTiers.map((tier) => ({
      upperBoundSize: tier.maxSizeBaseLots.toString(),
      maxLeverage: String(tier.maxLeverage),
      limitOrderRiskFactorBps: String(tier.limitOrderRiskFactor),
    })),
    riskFactors: {
      maintenanceMarginFactorBps: bps(
        market.riskFactors.maintenanceBps,
        market.riskFactors.maintenance
      ),
      backstopMarginFactorBps: bps(
        market.riskFactors.backstopBps,
        market.riskFactors.backstop
      ),
      highRiskMarginFactorBps: bps(
        market.riskFactors.highRiskBps,
        market.riskFactors.highRisk
      ),
    },
    cancelOrderRiskFactorBps: bps(
      market.riskFactors.cancelOrderBps,
      market.riskFactors.cancelOrder
    ),
    upnlRiskFactor: bps(market.riskFactors.upnlBps, market.riskFactors.upnl),
    upnlRiskFactorForWithdrawals: bps(
      market.riskFactors.upnlForWithdrawalsBps,
      market.riskFactors.upnlForWithdrawals
    ),
    isolatedOnly: market.isolatedOnly,
  });

  const liquidationTiers = new Set([
    "liquidatable",
    "backstopLiquidatable",
    "highRisk",
  ]);

  const estimateLiquidationPriceUsd = (
    basePositionLots: bigint,
    currentPriceUsd: number,
    crossesLiquidation: (priceUsd: number) => boolean
  ) => {
    if (basePositionLots === 0n || currentPriceUsd <= 0) return null;
    if (crossesLiquidation(currentPriceUsd)) return currentPriceUsd;

    if (basePositionLots > 0n) {
      let low = Math.max(currentPriceUsd / 1_000_000, Number.EPSILON);
      let high = currentPriceUsd;
      if (!crossesLiquidation(low)) return null;

      for (let i = 0; i < 48; i++) {
        const mid = (low + high) / 2;
        if (crossesLiquidation(mid)) low = mid;
        else high = mid;
      }
      return high;
    }

    let low = currentPriceUsd;
    let high = currentPriceUsd * 2;
    for (let i = 0; i < 20 && !crossesLiquidation(high); i++) {
      low = high;
      high *= 2;
    }
    if (!crossesLiquidation(high)) return null;

    for (let i = 0; i < 48; i++) {
      const mid = (low + high) / 2;
      if (crossesLiquidation(mid)) high = mid;
      else low = mid;
    }
    return high;
  };

  await client.exchange.ready();

  const marketData = client.marketData();
  const releaseMarketData = marketData.retain();
  await marketData.ready();

  const marginInputs = resource.marginInputs();
  const subaccount = marginInputs?.subaccounts.find(
    (item) => item.subaccountIndex === 0
  );
  if (!subaccount) throw new Error("subaccount margin inputs are not ready");

  const paramsBySymbol: Record<string, MarketParams> = {};
  for (const marketInput of subaccount.markets) {
    const market = client.exchange.market(marketInput.symbol);
    const markPrice = marketData.market(marketInput.symbol)?.markPrice;
    if (!market || markPrice === null || markPrice === undefined) {
      throw new Error(`missing cached market data for ${marketInput.symbol}`);
    }
    paramsBySymbol[marketInput.symbol] = paramsFromExchangeCache(market, markPrice);
  }

  const symbol = "SOL";
  const target = subaccount.markets.find((market) => market.symbol === symbol);
  const basePositionLots = BigInt(target?.position?.basePositionLots ?? "0");
  const currentPrice = marketData.market(symbol)?.markPrice;
  if (currentPrice === null || currentPrice === undefined) {
    throw new Error(`missing mark price for ${symbol}`);
  }

  const crossesLiquidation = (priceUsd: number) => {
    const market = client.exchange.market(symbol);
    if (!market) throw new Error(`unknown market ${symbol}`);

    const calculator = createMarginCalculator(
      Object.values({
        ...paramsBySymbol,
        [symbol]: paramsFromExchangeCache(market, priceUsd),
      })
    );
    const result = calculator.computeSubaccountMarginFromInputs(
      subaccount as SubaccountMarginInputs
    );
    return liquidationTiers.has(result.margin.riskTier);
  };

  const estimatedLiquidationPriceUsd = estimateLiquidationPriceUsd(
    basePositionLots,
    currentPrice,
    crossesLiquidation
  );

  console.log(estimatedLiquidationPriceUsd);
  releaseMarketData();
  ```

  ```rust Rust theme={null}
  use phoenix_rise::api::PhoenixMetadata;
  use phoenix_rise::math::{RiskTier, TraderPortfolio};

  fn estimate_liquidation_price_usd(
      portfolio: &TraderPortfolio,
      metadata: &PhoenixMetadata,
      symbol: &str,
      current_price: f64,
  ) -> Result<Option<f64>, String> {
      if current_price <= 0.0 {
          return Ok(None);
      }

      let symbol = symbol.to_ascii_uppercase();
      let Some(position) = portfolio.positions.get(&symbol) else {
          return Ok(None);
      };
      if position.is_neutral() {
          return Ok(None);
      }

      let calculator = *metadata
          .get_market_calculator(&symbol)
          .ok_or_else(|| format!("missing market calculator for {symbol}"))?;
      let mut provider = metadata.all_perp_asset_metadata().clone();

      let mut crosses_liquidation = |price: f64| -> Result<bool, String> {
          let mark_price = calculator
              .price_to_ticks(price)
              .map_err(|err| format!("{err:?}"))?;
          provider
              .get_mut(&symbol)
              .ok_or_else(|| format!("missing perp metadata for {symbol}"))?
              .set_mark_price(mark_price);

          let margin = portfolio
              .compute_margin(&provider)
              .map_err(|err| err.to_string())?;
          let tier = margin.risk_tier().map_err(|err| err.to_string())?;
          Ok(tier >= RiskTier::Liquidatable)
      };

      if crosses_liquidation(current_price)? {
          return Ok(Some(current_price));
      }

      if position.is_long() {
          let mut low = (current_price / 1_000_000.0).max(0.000001);
          let mut high = current_price;
          if !crosses_liquidation(low)? {
              return Ok(None);
          }

          for _ in 0..48 {
              let mid = (low + high) / 2.0;
              if crosses_liquidation(mid)? {
                  low = mid;
              } else {
                  high = mid;
              }
          }
          return Ok(Some(high));
      }

      let mut low = current_price;
      let mut high = current_price * 2.0;
      for _ in 0..20 {
          if crosses_liquidation(high)? {
              break;
          }
          low = high;
          high *= 2.0;
      }
      if !crosses_liquidation(high)? {
          return Ok(None);
      }

      for _ in 0..48 {
          let mid = (low + high) / 2.0;
          if crosses_liquidation(mid)? {
              high = mid;
          } else {
              low = mid;
          }
      }
      Ok(Some(high))
  }

  let cross = trader
      .primary_subaccount()
      .ok_or_else(|| "trader state is not ready".to_string())?;
  let portfolio = cross.to_trader_portfolio();
  let current_price = metadata
      .get_perp_asset_metadata("SOL")
      .and_then(|market| {
          metadata
              .get_market_calculator("SOL")
              .map(|calculator| calculator.ticks_to_price(market.mark_price))
      })
      .ok_or_else(|| "SOL market stats are not initialized".to_string())?;

  let estimated_liquidation_price_usd =
      estimate_liquidation_price_usd(&portfolio, &metadata, "SOL", current_price)?;

  println!("{estimated_liquidation_price_usd:?}");
  ```
</CodeGroup>

This estimate depends on the same cache freshness as margin monitoring. In TypeScript, keep `client.exchange` and `client.marketData()` live. In Rust, keep applying `PhoenixMetadata::apply_market_stats(&stats)` and trader-state updates before recomputing.

## Hawkeye Simulation

For an authoritative liquidation-price view, use Hawkeye simulation through the SDK. This runs a read-only simulation and decodes the program return data.

<CodeGroup>
  ```ts TypeScript theme={null}
  const margin = await client.rpc.hawkeye.viewMargin({
    authority: "AUTHORITY_PUBKEY",
    traderPdaIndex: 0,
    traderSubaccountIndex: 0,
  });

  const liquidation = await client.rpc.hawkeye.viewLiquidationPrice({
    authority: "AUTHORITY_PUBKEY",
    traderPdaIndex: 0,
    traderSubaccountIndex: 0,
    symbol: "SOL",
  });

  console.log(margin.returnData?.decoded);
  console.log(liquidation.returnData?.decoded.liquidationPriceTicks);
  ```

  ```rust Rust theme={null}
  use phoenix_rise::{PhoenixHawkeyeClient, TraderKey};

  let trader_key = TraderKey::new(authority);
  let hawkeye = PhoenixHawkeyeClient::new(
      "https://api.mainnet-beta.solana.com",
      &metadata,
  );

  let margin = hawkeye
      .view_margin_for_trader(trader_key.pda())
      .await?;
  let liquidation = hawkeye
      .view_liquidation_price_for_trader(trader_key.pda(), sol_asset_id)
      .await?;

  println!("margin={:?}", margin.value);
  println!(
      "liquidation price ticks={}",
      liquidation.value.liquidation_price_ticks
  );
  ```
</CodeGroup>

## Isolated Order Estimates

Server-built isolated order routes can also return the post-trade isolated liquidation estimate.

<CodeGroup>
  ```ts TypeScript theme={null}
  const result = await client.api.orders().placeIsolatedMarketOrderEnhanced({
    authority: "AUTHORITY_PUBKEY",
    symbol: "SOL",
    side: "buy",
    numBaseLots: 67,
    transferAmount: 100_000_000,
    allowCrossAndIsolatedForAsset: true,
  });

  console.log(result.estimatedLiquidationPriceUsd);
  ```

  ```rust Rust theme={null}
  let (ixs, estimated_liq) = http
      .orders()
      .build_isolated_market_order_tx_enhanced(
          &authority,
          "SOL",
          Side::Bid,
          67,
          Some(100_000_000),
          true,
          None,
      )
      .await?;

  println!("estimated liquidation price: {:?}", estimated_liq);
  ```
</CodeGroup>

<!-- END SOURCE: sdk/margin.md -->

---

# Source: `sdk/markets.md`

**Original URL:** `https://docs.phoenix.trade/sdk/markets.md`

**Local path:** `sdk/markets.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Exchange Data

> Use the Rise SDK exchange cache, market metadata, market stats, and orderbook streams.

Use this page when your integration needs market params, public market metadata, live mark prices, L2 orderbooks, or commodity/equity calendar state.

The exchange cache should be the default source for market params in both SDKs. It resolves the PerpAssetMap, global exchange accounts, market accounts, spline accounts, fee params, risk params, and public market metadata such as logos. Use REST to bootstrap, then let the SDK keep a local cache fresh instead of repeatedly polling the same params.

## Exchange Cache

In TypeScript, configure `createPhoenixClient(...)` with exchange metadata streaming, wait for `client.exchange.ready()`, then read markets from the cache.

<CodeGroup>
  ```ts TypeScript theme={null}
  import { createPhoenixClient } from "@ellipsis-labs/rise";

  const client = createPhoenixClient({
    apiUrl: "https://perp-api.phoenix.trade",
    rpcUrl: "https://api.mainnet-beta.solana.com",
    ws: { connectMode: "eager" },
    exchangeMetadata: { stream: true },
  });

  const snapshot = await client.exchange.ready();
  const sol = client.exchange.market("SOL");
  const solMetadata = client.exchange.marketMetadata("SOL");

  console.log(snapshot.markets.length);
  console.log(sol?.assetId, sol?.tickSize, sol?.baseLotsDecimals);
  console.log(solMetadata?.logoUri);

  const unsubscribe = client.exchange.onEvent((event) => {
    if (event.type === "marketUpdated") {
      console.log(event.symbol, event.change);
    }
  });
  ```

  ```rust Rust theme={null}
  use phoenix_rise::{PhoenixHttpClient, PhoenixMetadata};

  let http = PhoenixHttpClient::new_from_env()?;
  let exchange = http.get_exchange().await?.into();
  let mut metadata = PhoenixMetadata::new(exchange);

  let sol = metadata
      .get_market("SOL")
      .expect("SOL market should exist in exchange metadata");

  println!("asset id: {}", sol.asset_id);
  println!("market: {}", sol.market_pubkey);
  println!("spline: {}", sol.spline_pubkey);
  ```
</CodeGroup>

The TypeScript order-packet builders and `client.ixs` instruction builders read from `client.exchange`. The Rust `PhoenixTxBuilder` reads from `PhoenixMetadata`. Keep those caches warm before building orders so price, tick, base-lot, account, and risk-param conversions use the same market view as the rest of your app.

References:

* [TypeScript unified client example](https://github.com/Ellipsis-Labs/rise-public/blob/master/ts/examples/phoenix-client-example.ts)
* [TypeScript limit-order ix example](https://github.com/Ellipsis-Labs/rise-public/blob/master/ts/examples/03-build-limit-order-ix.ts)
* [Rust HTTP client example](https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/sdk/examples/http_client.rs)
* [Rust limit-order example](https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/sdk/examples/send_limit_order.rs)

## Market Stats

Market stats carry live mark price, mid price, oracle price, funding, volume, and open interest. Subscribe for current values; do not poll market stats in a tight loop.

<CodeGroup>
  ```ts TypeScript theme={null}
  const controller = new AbortController();

  for await (const update of client.streams!.marketStats("SOL", controller.signal)) {
    console.log(
      update.symbol,
      update.stats.markPrice,
      update.stats.currentFundingRate
    );
  }
  ```

  ```rust Rust theme={null}
  use phoenix_rise::api::{PhoenixMetadata, PhoenixWSClient};

  let ws = PhoenixWSClient::new_from_env()?;
  let (mut rx, _handle) = ws.subscribe_to_market("SOL".to_string())?;

  while let Some(stats) = rx.recv().await {
      metadata.apply_market_stats(&stats)?;
      println!("{} mark=${}", stats.symbol, stats.mark_price);
  }
  ```
</CodeGroup>

In Rust, `metadata.apply_market_stats(&stats)` updates the cached mark price inputs used by margin calculations.

The TypeScript stream adapter converts the raw WebSocket payload into camelCase. `timestamp` is a `bigint` in TypeScript and is shown as a string below for display:

```json theme={null}
{
  "symbol": "SOL",
  "stats": {
    "timestamp": "1719868800000",
    "openInterest": 1000.0,
    "markPrice": 150.1,
    "oraclePrice": 150.0,
    "prevDayMarkPrice": 148.7,
    "dayVolumeUsd": 2500000.0,
    "dayVolumeBase": 16655.1,
    "currentFundingRate": 0.0000125,
    "eightHourFundingRate": 0.0001,
    "annualizedFundingRate": 0.1095
  }
}
```

For raw WebSocket payloads, see [WebSocket market stats](/api/websocket#market-stats). For historical stats, use the REST route [`GET /v1/market/{symbol}/stats`](/api/get-market-stats-history).

References:

* [TypeScript WebSocket example](https://github.com/Ellipsis-Labs/rise-public/blob/master/ts/examples/phoenix-ws-example.ts)
* [Rust market-stats example](https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/sdk/examples/subscribe_market_stats.rs)

## L2 Orderbook

Use the L2 stream when you need current book levels. Keep one subscription per symbol and fan out internally to your UI or strategy modules.

<CodeGroup>
  ```ts TypeScript theme={null}
  const controller = new AbortController();

  for await (const update of client.streams!.l2Book("SOL", controller.signal)) {
    console.log(update.symbol, update.bids[0], update.asks[0]);
  }
  ```

  ```rust Rust theme={null}
  use phoenix_rise::api::PhoenixWSClient;
  use phoenix_rise::types::l2book::L2Book;

  let ws = PhoenixWSClient::new_from_env()?;
  let (mut rx, _handle) =
      ws.subscribe_to_orderbook_with_options("SOL".to_string(), false)?;
  let mut book = L2Book::new("SOL".to_string());

  while let Some(update) = rx.recv().await {
      book.apply_update(&update);
      println!("bid={:?} ask={:?}", book.best_bid(), book.best_ask());
  }
  ```
</CodeGroup>

Reference: [Rust L2 book example](https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/sdk/examples/subscribe_l2_book.rs).

## API Polling

Polling is appropriate for startup snapshots, periodic reconciliation, or services that do not need live updates. It is not a substitute for WebSocket streams in a trading UI or market-making service.

<CodeGroup>
  ```ts TypeScript theme={null}
  const snapshot = await client.api.exchange().getSnapshot();
  const markets = await client.api.markets().getMarkets();
  const sol = await client.api.markets().getMarket("SOL");

  console.log(snapshot.markets.length, markets.length, sol.symbol);
  ```

  ```rust Rust theme={null}
  let exchange = http.get_exchange().await?.into();
  let metadata = PhoenixMetadata::new(exchange);
  let markets = http.markets().get_markets().await?;
  let sol = http.markets().get_market("SOL").await?;

  println!("{} {}", metadata.exchange().markets.len(), markets.len());
  println!("{}", sol.symbol);
  ```
</CodeGroup>

If you need to poll exchange params, poll on a slow interval and replace the local cache atomically. In TypeScript, prefer `exchangeMetadata: { stream: true }` when WebSocket access is available. In Rust, rebuild `PhoenixMetadata` from a fresh `get_exchange()` snapshot after reconnects or scheduled reconciliation, then continue applying market stats.

## Market Metadata And Calendars

The exchange cache includes public market metadata used by apps, including labels and logo URIs where available. Treat these fields as display metadata; trading logic should use the market params and risk params from the cache.

Commodity and equity markets can have session windows, after-hours bounds, reopen windows, and stale index behavior. The exchange cache exposes lightweight calendar metadata with the rest of the market snapshot so your UI can show the market's current calendar source and the next known transition without another API call.

<CodeGroup>
  ```ts TypeScript theme={null}
  await client.exchange.ready();

  const sol = client.exchange.market("SOL");
  const publicMetadata = client.exchange.marketMetadata("SOL");
  const cachedCalendar = sol?.metadata?.calendar ?? publicMetadata?.calendar;

  console.log(publicMetadata?.logoUri);
  console.log(cachedCalendar?.id);
  console.log(cachedCalendar?.calendarUri);
  console.log(cachedCalendar?.nextMarketTransitionUtc);
  ```

  ```rust Rust theme={null}
  let sol = metadata
      .get_market("SOL")
      .expect("SOL market should exist in exchange metadata");

  let cached_calendar = sol
      .metadata
      .as_ref()
      .and_then(|metadata| metadata.calendar.as_ref());

  println!("{:?}", sol.metadata.as_ref().and_then(|m| m.logo_uri.as_ref()));
  println!("{:?}", cached_calendar.map(|calendar| &calendar.id));
  println!("{:?}", cached_calendar.map(|calendar| &calendar.calendar_uri));
  println!("{:?}", cached_calendar.and_then(|calendar| {
      calendar.next_market_transition_utc.as_ref()
  }));
  ```
</CodeGroup>

Use the API clients when you need the full weekly schedule, date overrides, or current/next open state computed by the API.

<CodeGroup>
  ```ts TypeScript theme={null}
  const calendar = await client.api.markets().getMarketCalendar("SOL");
  const nextTransition =
    await client.api.markets().getNextMarketCalendarTransition("SOL");
  const nextCommodityTransition =
    await client.api.markets().getNextCommodityMarketTransition();
  const commodityCalendar =
    await client.api.markets().getCommodityMarketCalendar();

  console.log(calendar.marketCalendarId, calendar.calendar?.weeklySchedule);
  console.log(nextTransition.currentState, nextTransition.utcNextTransition);
  console.log(nextCommodityTransition.market, nextCommodityTransition.nextMarketState);
  console.log(commodityCalendar.calendar.weeklySchedule);

  if (calendar.marketCalendarId) {
    const calendarRecord =
      await client.api.markets().getMarketCalendarById(calendar.marketCalendarId);
    console.log(calendarRecord.calendar?.weeklySchedule);
  }
  ```

  ```rust Rust theme={null}
  let calendar = http.markets().get_market_calendar("SOL").await?;
  let next_transition = http
      .markets()
      .get_next_market_calendar_transition("SOL")
      .await?;
  let next_commodity_transition = http
      .markets()
      .get_next_commodity_market_transition()
      .await?;
  let commodity_calendar = http.markets().get_commodity_market_calendar().await?;

  println!("{}", calendar.market_calendar_id);
  println!("{:?}", calendar.calendar.as_ref().map(|c| &c.weekly_schedule));
  println!("{:?}", next_transition.current_state);
  println!("{:?}", next_transition.utc_next_transition);
  println!("{:?}", next_commodity_transition.utc_next_transition);
  println!("{:?}", commodity_calendar.calendar.weekly_schedule);
  ```
</CodeGroup>

TypeScript also exposes `getMarketCalendarById(id)` when you start from the calendar id stored in exchange metadata. Rust currently exposes the symbol and commodity calendar helpers.

API reference:

* [`GET /v1/market/{symbol}/market-calendar`](/api/get-market-calendar)
* [`GET /v1/market-calendar/{market_calendar_id}`](/api/get-market-calendar-by-id)
* [`GET /v1/market/{symbol}/next-market-calendar-transition`](/api/get-next-market-calendar-transition)
* [`GET /v1/market/next-commodity-market-transition`](/api/get-next-commodity-market-transition)
* [`GET /v1/market/commodity-calendar`](/api/get-commodity-market-calendar)

Related Phoenix docs:

* [RWA index price](/phoenix/real-world-assets/index-price)
* [RWA mark price and bounds](/phoenix/real-world-assets/mark-price-and-bounds)
* [Market specs](/phoenix/real-world-assets/market-specs)

<!-- END SOURCE: sdk/markets.md -->

---

# Source: `sdk/on-chain-programs.md`

**Original URL:** `https://docs.phoenix.trade/sdk/on-chain-programs.md`

**Local path:** `sdk/on-chain-programs.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# On-Chain Programs

> Integrate Solana programs with Phoenix perpetuals through the Rise Rust SDK and CPI helpers.

Use the Rise Rust CPI surface when your Solana program already has `AccountInfo`
handles and needs to invoke Phoenix, Ember, Flight, or Hawkeye directly. The SDK
does not fetch accounts or derive every PDA at CPI time. Your program validates
and resolves accounts, then adapts them into typed CPI contexts such as
`phoenix::PlaceMarketOrder`, `phoenix::PlaceStopLoss`, or
`hawkeye::ViewMargin`.

Reference implementations:

* [rise-public programs/example-program](https://github.com/Ellipsis-Labs/rise-public/tree/master/programs/example-program)
* [example-program/src/market.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/src/market.rs)
* [example-program/src/place\_stop\_loss.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/src/place_stop_loss.rs)
* [example-program/tests/common/mod.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/tests/common/mod.rs)

## Example Program Map

Use the example program as the source of truth for complete account lists. The
docs below show the shape of each CPI, but the linked files show the full outer
instruction, parameter struct, account metas, LiteSVM test setup, and log
assertions.

| Flow                                                                                                 | Program code                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Test coverage                                                                                                                       |
| ---------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| Market order, limit order, cancel by id, matching-engine return data, and Hawkeye margin read        | [market.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/src/market.rs), [place\_market\_order.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/src/place_market_order.rs), [place\_limit\_order.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/src/place_limit_order.rs), [cancel\_limit\_order.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/src/cancel_limit_order.rs) | [market\_orders.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/tests/market_orders.rs)       |
| Ember wrap then Phoenix deposit                                                                      | [deposit\_ember\_then\_phoenix.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/src/deposit_ember_then_phoenix.rs)                                                                                                                                                                                                                                                                                                                                                                            | [deposit\_withdraw.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/tests/deposit_withdraw.rs) |
| Phoenix withdraw then Ember unwrap                                                                   | [withdraw\_phoenix\_then\_ember.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/src/withdraw_phoenix_then_ember.rs)                                                                                                                                                                                                                                                                                                                                                                          | [deposit\_withdraw.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/tests/deposit_withdraw.rs) |
| Stop-loss placement and cancellation                                                                 | [place\_stop\_loss.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/src/place_stop_loss.rs), [cancel\_stop\_loss.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/src/cancel_stop_loss.rs)                                                                                                                                                                                                                                                               | [stop\_loss.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/tests/stop_loss.rs)               |
| Register isolated subaccount, sync parent capabilities, transfer collateral, trade, close, and sweep | [register\_subaccount\_sync\_transfer\_and\_market\_order.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/src/register_subaccount_sync_transfer_and_market_order.rs), [close\_subaccount\_position\_and\_sweep.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/src/close_subaccount_position_and_sweep.rs)                                                                                                                                             | [subaccounts.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/tests/subaccounts.rs)            |
| Shared fixture setup, dynamic accounts, account metas, compute budget, and log helpers               | [tests/common/mod.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/tests/common/mod.rs)                                                                                                                                                                                                                                                                                                                                                                                                       | [programs/example-program/tests](https://github.com/Ellipsis-Labs/rise-public/tree/master/programs/example-program/tests)           |

## Dependencies

Most on-chain programs should depend on the `phoenix-rise` facade with only the
`cpi` feature enabled. That profile exposes account byte decoders, instruction
layouts, and Pinocchio CPI helpers without the HTTP, WebSocket, RPC, or
transaction-builder graph.

```toml Cargo theme={null}
[dependencies]
phoenix-rise = { version = "0.3", default-features = false, features = ["cpi"] }
pinocchio = "0.9.4"

# Optional, but used by the public example program for instruction params,
# program ids, and program-id validation helpers.
borsh = { version = "1.6", features = ["derive"] }
pinocchio-pubkey = "0.3"
solana-pubkey = { version = "~3.0", default-features = false }
```

If you only want the instruction and CPI layer, depend on `phoenix-rise-ix`
directly.

```toml Cargo theme={null}
[dependencies]
phoenix-rise-ix = { version = "0.3", default-features = false, features = ["cpi"] }
pinocchio = "0.9.4"
```

The public example program uses the facade from the local workspace:

```toml Cargo theme={null}
phoenix-rise = { path = "../../rust/sdk", default-features = false, features = [
  "cpi",
] }
```

Source: [example-program/Cargo.toml](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/Cargo.toml).

## Imports

Using the facade crate:

```rust Rust theme={null}
use phoenix_rise::ix;
use phoenix_rise::ix::types::{OrderFlags, SelfTradeBehavior, Side};
use pinocchio::account_info::AccountInfo;
use pinocchio::program_error::ProgramError;
use pinocchio::{ProgramResult, msg};
```

Using the low-level instruction crate directly:

```rust Rust theme={null}
use phoenix_rise_ix::cpi::{CpiScratch, hawkeye, phoenix};
use phoenix_rise_ix::types::{OrderFlags, SelfTradeBehavior, Side};
use pinocchio::account_info::AccountInfo;
use pinocchio::program_error::ProgramError;
```

## CPI Shape

The typed CPI contexts are account-context structs. They borrow accounts from
your outer instruction and encode the expected Phoenix account order for you.
They do not own accounts and they do not resolve missing accounts from chain
state.

Phoenix order CPIs usually need:

* Phoenix program id and log authority.
* Global config and `PerpAssetMap`.
* Trader signer or delegated authority, plus the trader account.
* Market orderbook and spline collection.
* Dynamic `global_trader_index` accounts.
* Dynamic `active_trader_buffer` accounts.
* Optional accounts for stop losses, conditionals, Hawkeye, Ember, or Flight.

The example program keeps a fixed account prefix and passes the dynamic
`global_trader_index` and `active_trader_buffer` accounts at the tail. The tail
counts are carried in instruction data, so the loader can split the account
slice without allocation.

```rust Rust theme={null}
pub(crate) fn dynamic_tail<'a>(
    accounts: &'a [AccountInfo],
    fixed_account_count: usize,
    global_trader_index_count: usize,
    active_trader_buffer_count: usize,
) -> Result<(&'a [AccountInfo], &'a [AccountInfo]), ProgramError> {
    let gti_end = fixed_account_count
        .checked_add(global_trader_index_count)
        .ok_or(ProgramError::InvalidInstructionData)?;
    let expected = gti_end
        .checked_add(active_trader_buffer_count)
        .ok_or(ProgramError::InvalidInstructionData)?;

    require_exact_accounts(accounts, expected)?;
    Ok((
        &accounts[fixed_account_count..gti_end],
        &accounts[gti_end..expected],
    ))
}
```

Source: [example-program/src/common.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/src/common.rs).

## Market Context

A useful integration pattern is to create one outer context that knows how to
load and validate your instruction accounts, then add small helper methods that
project those accounts into the SDK's typed CPI contexts.

```rust Rust theme={null}
use phoenix_rise::ix;
use pinocchio::account_info::AccountInfo;
use pinocchio::program_error::ProgramError;
use pinocchio::{ProgramResult, msg};

use crate::common::dynamic_tail;
use crate::cpi::check_program_id;

pub(crate) struct MarketContext<'a> {
    phoenix_program: &'a AccountInfo,
    hawkeye_program: &'a AccountInfo,
    log_authority: &'a AccountInfo,
    global_config: &'a AccountInfo,
    trader: &'a AccountInfo,
    trader_account: &'a AccountInfo,
    perp_asset_map: &'a AccountInfo,
    orderbook: &'a AccountInfo,
    spline_collection: &'a AccountInfo,
    global_trader_index: &'a [AccountInfo],
    active_trader_buffer: &'a [AccountInfo],
}

impl<'a> MarketContext<'a> {
    const FIXED_ACCOUNT_COUNT: usize = 9;

    pub(crate) fn load(
        accounts: &'a [AccountInfo],
        global_trader_index_count: usize,
        active_trader_buffer_count: usize,
    ) -> Result<Self, ProgramError> {
        let (global_trader_index, active_trader_buffer) = dynamic_tail(
            accounts,
            Self::FIXED_ACCOUNT_COUNT,
            global_trader_index_count,
            active_trader_buffer_count,
        )?;

        let context = Self {
            phoenix_program: &accounts[0],
            hawkeye_program: &accounts[1],
            log_authority: &accounts[2],
            global_config: &accounts[3],
            trader: &accounts[4],
            trader_account: &accounts[5],
            perp_asset_map: &accounts[6],
            orderbook: &accounts[7],
            spline_collection: &accounts[8],
            global_trader_index,
            active_trader_buffer,
        };
        context.validate()?;
        Ok(context)
    }

    fn validate(&self) -> ProgramResult {
        check_program_id(self.phoenix_program, &ix::PHOENIX_PROGRAM_ID, "Phoenix")?;
        check_program_id(self.hawkeye_program, &ix::HAWKEYE_PROGRAM_ID, "Hawkeye")?;
        if !self.trader.is_signer() {
            msg!("trader must sign market action");
            return Err(ProgramError::MissingRequiredSignature);
        }
        Ok(())
    }

    fn place_market_order_accounts(&self) -> ix::cpi::phoenix::PlaceMarketOrder<'a> {
        ix::cpi::phoenix::PlaceMarketOrder {
            phoenix_program: self.phoenix_program,
            log_authority: self.log_authority,
            global_config: self.global_config,
            trader: self.trader,
            trader_account: self.trader_account,
            perp_asset_map: self.perp_asset_map,
            global_trader_index: self.global_trader_index,
            active_trader_buffer: self.active_trader_buffer,
            orderbook: self.orderbook,
            spline_collection: self.spline_collection,
        }
    }
}
```

Source: [example-program/src/market.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/src/market.rs).

## Invoke Phoenix

Once your outer context can build the typed CPI context, the invoke step is
small: create `CpiScratch`, pass instruction-specific args, and let the SDK
write the account metas and instruction data.

```rust Rust theme={null}
use crate::cpi::MAX_CPI_ACCOUNTS;
use crate::params::MarketOrderCpiParams;

impl<'a> MarketContext<'a> {
    pub(crate) fn invoke_market_order(
        &self,
        params: &MarketOrderCpiParams,
    ) -> ProgramResult {
        let market_order = self.place_market_order_accounts();

        let mut scratch = ix::cpi::CpiScratch::<
            { MAX_CPI_ACCOUNTS },
            { ix::cpi::phoenix::PlaceMarketOrder::MAX_DATA_LEN },
        >::new(self.phoenix_program);

        market_order.invoke(
            ix::cpi::phoenix::PlaceMarketOrderArgs {
                side: params.side,
                price_in_ticks: params.price_in_ticks,
                num_base_lots: params.num_base_lots,
                num_quote_lots: params.num_quote_lots,
                min_base_lots_to_fill: params.min_base_lots_to_fill,
                min_quote_lots_to_fill: params.min_quote_lots_to_fill,
                self_trade_behavior: params.self_trade_behavior,
                match_limit: params.match_limit,
                client_order_id: params.client_order_id,
                last_valid_slot: params.last_valid_slot,
                order_flags: params.order_flags,
                cancel_existing: params.cancel_existing,
            },
            &mut scratch,
        )
    }
}
```

`CpiScratch` owns fixed-size stack buffers for account infos, account metas, and
instruction data. For dynamic market accounts, size it to your integration's
upper bound and check `ctx.account_count()` before invoking if the account count
is user-controlled. If you already have reusable storage, use `CpiBuffers` and
`invoke_with_buffers(...)` instead.

## Stop Losses And Conditionals

Stop losses and conditional orders follow the same pattern, but the account
context includes the funder, position authority, conditional or stop-loss
account, and system program. Creating a stop-loss or conditional-order account
can incur rent if the account does not already exist.

```rust Rust theme={null}
let place_stop_loss = ix::cpi::phoenix::PlaceStopLoss {
    phoenix_program,
    log_authority,
    global_config,
    funder,
    trader_account,
    perp_asset_map,
    global_trader_index,
    active_trader_buffer,
    orderbook,
    spline_collection,
    position_authority,
    stop_loss_account,
    system_program,
};

let mut scratch = ix::cpi::CpiScratch::<
    { MAX_CPI_ACCOUNTS },
    { ix::cpi::phoenix::PlaceStopLoss::DATA_LEN },
>::new(phoenix_program);

place_stop_loss.invoke(
    ix::cpi::phoenix::PlaceStopLossArgs {
        trigger_price,
        execution_price,
        trade_side,
        execution_direction,
        order_kind,
    },
    &mut scratch,
)?;
```

Source: [example-program/src/place\_stop\_loss.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/src/place_stop_loss.rs).

## Collateral CPIs

Collateral flows often compose Ember and Phoenix CPIs in one outer instruction.
The example program wraps fake USDC into Phoenix collateral through Ember, then
deposits that collateral into the Phoenix trader account.

```rust Rust theme={null}
let ember_deposit = ix::cpi::ember::EmberDeposit {
    ember_program,
    trader,
    ember_state,
    usdc_mint,
    canonical_mint,
    trader_usdc_account,
    trader_phoenix_account,
    ember_vault,
    token_program,
};
let mut ember_scratch = ix::cpi::CpiScratch::<
    { ix::cpi::ember::EmberDeposit::ACCOUNT_COUNT },
    { ix::cpi::ember::EmberDeposit::DATA_LEN },
>::new(ember_program);
ember_deposit.invoke(
    ix::cpi::ember::EmberDepositArgs { amount: ember_amount },
    &mut ember_scratch,
)?;

let phoenix_deposit = ix::cpi::phoenix::PhoenixDeposit {
    phoenix_program,
    log_authority,
    global_config,
    trader,
    trader_token_account: trader_phoenix_account,
    trader_account,
    global_vault,
    token_program,
    global_trader_index,
    active_trader_buffer,
    permission_account: None,
};
let mut phoenix_scratch = ix::cpi::CpiScratch::<
    { MAX_CPI_ACCOUNTS },
    { ix::cpi::phoenix::PhoenixDeposit::DATA_LEN },
>::new(phoenix_program);
phoenix_deposit.invoke(
    ix::cpi::phoenix::PhoenixDepositArgs { amount: phoenix_amount },
    &mut phoenix_scratch,
)?;
```

Source: [example-program/src/deposit\_ember\_then\_phoenix.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/src/deposit_ember_then_phoenix.rs).

Withdrawals reverse the sequence: Phoenix withdraw first, then Ember unwrap.
Source: [example-program/src/withdraw\_phoenix\_then\_ember.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/src/withdraw_phoenix_then_ember.rs).

## Subaccount CPIs

Subaccount flows are a good example of composing several typed contexts. The
example registers the child trader account, syncs parent capabilities, transfers
collateral to the child, then reuses `MarketContext` to place the child order.

```rust Rust theme={null}
register.invoke(
    ix::cpi::phoenix::RegisterTraderArgs {
        max_positions,
        trader_pda_index,
        subaccount_index: child_subaccount_index,
    },
    &mut register_scratch,
)?;

sync.invoke(
    ix::cpi::phoenix::SyncParentToChildArgs,
    &mut sync_scratch,
)?;

transfer.invoke(
    ix::cpi::phoenix::TransferCollateralArgs { amount },
    &mut transfer_scratch,
)?;

let market_context = MarketContext::from_refs(MarketAccountRefs {
    phoenix_program,
    hawkeye_program,
    log_authority,
    global_config,
    trader: trader_authority,
    trader_account: child_trader_account,
    perp_asset_map,
    orderbook,
    spline_collection,
    global_trader_index,
    active_trader_buffer,
})?;
market_context.invoke_market_order_without_arenas(&order, "subaccount-open-order")?;
```

Source: [example-program/src/register\_subaccount\_sync\_transfer\_and\_market\_order.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/src/register_subaccount_sync_transfer_and_market_order.rs).

## Return Data

Phoenix order CPIs can return matching-engine data. Decode return data instead
of parsing logs.

```rust Rust theme={null}
use phoenix_rise::ix::return_data::decode_matching_engine_cpi_response;
use pinocchio::cpi::get_return_data;

let Some(return_data) = get_return_data() else {
    return Ok(());
};

let response = decode_matching_engine_cpi_response(return_data.as_slice())?;
if let Some(order_id) = response.order_id() {
    msg!(&format!(
        "order_id_price_in_ticks={} order_sequence_number={}",
        order_id.price_in_ticks,
        order_id.order_sequence_number,
    ));
}
```

Source: [example-program/src/common.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/src/common.rs).

## Hawkeye Views

Hawkeye is the read-only program for authoritative margin, liquidation-price,
BBO, and funding views. Invoke it when local SDK math is not enough and you need
the result the on-chain programs would use. Hawkeye writes versioned return data;
decode those bytes instead of parsing logs.

Hawkeye program id: `RiSeVw3ZjNfsaXPRb4mgaqYaEEt41pNNJoDvVh7pgQj`.
Use it to validate the CPI program account on-chain and to assert that
simulation return data came from Hawkeye off-chain.

<CodeGroup>
  ```ts TypeScript theme={null}
  import { HAWKEYE_PROGRAM_ADDRESS } from "@ellipsis-labs/rise";

  const result = await client.rpc.hawkeye.viewMargin({
    authority: "AUTHORITY_PUBKEY",
    traderPdaIndex: 0,
    traderSubaccountIndex: 0,
  });

  if (result.returnData?.programId !== HAWKEYE_PROGRAM_ADDRESS) {
    throw new Error("unexpected Hawkeye return-data program");
  }
  ```

  ```rust Rust theme={null}
  use phoenix_rise::ix;
  use pinocchio::account_info::AccountInfo;
  use pinocchio::ProgramResult;

  use crate::cpi::check_program_id;

  fn validate_hawkeye_program(hawkeye_program: &AccountInfo) -> ProgramResult {
      check_program_id(hawkeye_program, &ix::HAWKEYE_PROGRAM_ID, "Hawkeye")
  }
  ```
</CodeGroup>

Hawkeye instructions are normal Solana instructions, so they can be invoked
off-chain in a transaction simulation or on-chain through CPI. The current typed
Pinocchio CPI helper in the Rust SDK is `ix::cpi::hawkeye::ViewMargin`; the
other views are exposed through off-chain instruction builders and can be
mirrored on-chain with the same account metas and discriminators if your program
needs them.

| View              | Hawkeye ix               | Return data                                                    | SDK entry points                                                                                                                                                                                                    |
| ----------------- | ------------------------ | -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Account margin    | `view_margin`            | `ViewMarginReturn` / `HawkeyeMarginReturn`                     | On-chain `ix::cpi::hawkeye::ViewMargin`; TypeScript `client.rpc.hawkeye.viewMargin(...)` or `buildHawkeyeViewMarginIx(...)`; Rust `PhoenixHawkeyeClient::view_margin(...)` or `create_hawkeye_view_margin_ix(...)`  |
| Asset margin      | `view_margin_for_asset`  | `ViewAssetReturn` / `HawkeyeAssetReturn`                       | TypeScript `client.rpc.hawkeye.viewMarginForAsset(...)` or `buildHawkeyeViewMarginForAssetIx(...)`; Rust `PhoenixHawkeyeClient::view_margin_for_asset(...)` or `create_hawkeye_view_margin_for_asset_ix(...)`       |
| Liquidation price | `view_liquidation_price` | `ViewLiquidationPriceReturn` / `HawkeyeLiquidationPriceReturn` | TypeScript `client.rpc.hawkeye.viewLiquidationPrice(...)` or `buildHawkeyeViewLiquidationPriceIx(...)`; Rust `PhoenixHawkeyeClient::view_liquidation_price(...)` or `create_hawkeye_view_liquidation_price_ix(...)` |
| Best bid/offer    | `view_bbo`               | `ViewBboReturn` / `HawkeyeBboReturn`                           | TypeScript `client.rpc.hawkeye.viewBbo(...)` or `buildHawkeyeViewBboIx(...)`; Rust `PhoenixHawkeyeClient::view_bbo(...)` or `create_hawkeye_view_bbo_ix(...)`                                                       |
| Funding           | `view_funding`           | `ViewFundingReturn` / `HawkeyeFundingReturn`                   | TypeScript `client.rpc.hawkeye.viewFunding(...)` or `buildHawkeyeViewFundingIx(...)`; Rust `PhoenixHawkeyeClient::view_funding(...)` or `create_hawkeye_view_funding_ix(...)`                                       |

Sources: [rust/ix/src/hawkeye.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/ix/src/hawkeye.rs),
[ts/src/hawkeye.ts](https://github.com/Ellipsis-Labs/rise-public/blob/master/ts/src/hawkeye.ts),
[rust/core/src/hawkeye\_client.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/core/src/hawkeye_client.rs),
and [ts/src/rpc.ts](https://github.com/Ellipsis-Labs/rise-public/blob/master/ts/src/rpc.ts).

### On-chain CPI

On-chain programs read Hawkeye return data the same way they read Phoenix return
data: invoke the program, call `get_return_data()` immediately, assert the
return-data program id is Hawkeye, then decode the expected struct. Return data
is overwritten by the next CPI that sets it, so decode or copy it before another
Phoenix or Hawkeye call.

The example program invokes `hawkeye::ViewMargin` after market orders and
decodes the return into `ViewMarginReturn`.

```rust Rust theme={null}
let view_margin = ix::cpi::hawkeye::ViewMargin {
    hawkeye_program,
    phoenix_program,
    global_config,
    global_trader_index,
    active_trader_buffer,
    perp_asset_map,
    trader: trader_account,
};

let mut scratch = ix::cpi::CpiScratch::<
    { MAX_CPI_ACCOUNTS },
    { ix::cpi::hawkeye::ViewMargin::DATA_LEN },
>::new(hawkeye_program);

let margin = view_margin.invoke_and_decode(&mut scratch)?;
msg!(&format!(
    "collateral={} free={} maintenance={} liquidatable={}",
    margin.collateral_quote_lots,
    margin.free_collateral_quote_lots,
    margin.maintenance_margin_quote_lots,
    margin.is_liquidatable,
));
```

If you need the manual decode path, use the same guard the helper uses
internally.

```rust Rust theme={null}
use phoenix_rise::ix::hawkeye::{ViewMarginReturn, decode_hawkeye_return};
use pinocchio::cpi::get_return_data;

let return_data = get_return_data().ok_or(ProgramError::InvalidAccountData)?;
if return_data.program_id() != &ix::HAWKEYE_PROGRAM_ID.to_bytes() {
    return Err(ProgramError::InvalidAccountData);
}

let margin = decode_hawkeye_return::<ViewMarginReturn>(
    return_data.as_slice(),
    "view_margin",
)
.map_err(|_| ProgramError::InvalidAccountData)?;
```

Source: [example-program/src/market.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/programs/example-program/src/market.rs)
and [rust/ix/src/cpi.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/ix/src/cpi.rs).

### Off-chain simulation

For applications and services, prefer the SDK Hawkeye RPC helpers. They build a
read-only simulation transaction, validate that return data came from the
Hawkeye program id, and decode the bytes into the typed return shape.

<CodeGroup>
  ```ts TypeScript theme={null}
  const margin = await client.rpc.hawkeye.viewMargin({
    authority: "AUTHORITY_PUBKEY",
    traderPdaIndex: 0,
    traderSubaccountIndex: 0,
  });

  const liquidation = await client.rpc.hawkeye.viewLiquidationPrice({
    authority: "AUTHORITY_PUBKEY",
    traderPdaIndex: 0,
    traderSubaccountIndex: 0,
    symbol: "SOL",
  });

  const bbo = await client.rpc.hawkeye.viewBbo({ symbol: "SOL" });

  console.log(margin.returnData?.decoded.freeCollateralQuoteLots);
  console.log(liquidation.returnData?.decoded.liquidationPriceTicks);
  console.log(bbo.returnData?.decoded.markPriceTicks);
  ```

  ```rust Rust theme={null}
  use phoenix_rise::{PhoenixHawkeyeClient, TraderKey};

  let trader_key = TraderKey::new(authority);
  let hawkeye = PhoenixHawkeyeClient::new(rpc_url, &metadata);

  let margin = hawkeye
      .view_margin_for_trader(trader_key.pda())
      .await?;
  let liquidation = hawkeye
      .view_liquidation_price_for_trader(trader_key.pda(), sol_asset_id)
      .await?;
  let bbo = hawkeye.view_bbo("SOL").await?;

  println!("free collateral={}", margin.value.free_collateral_quote_lots);
  println!(
      "liquidation price ticks={}",
      liquidation.value.liquidation_price_ticks
  );
  println!("mark price ticks={}", bbo.value.mark_price_ticks);
  ```
</CodeGroup>

If you already have accounts resolved, you can build a Hawkeye instruction
directly and still let the SDK decode the return data.

<CodeGroup>
  ```ts TypeScript theme={null}
  import { buildHawkeyeViewLiquidationPriceIx } from "@ellipsis-labs/rise";

  const ix = buildHawkeyeViewLiquidationPriceIx({
    phoenixProgramAddress,
    globalConfigurationAddress,
    globalTraderIndex,
    activeTraderBuffer,
    perpAssetMap,
    traderAccount,
    assetId: 0,
  });

  const simulation = await client.rpc.hawkeye.simulateInstruction(ix);
  const decoded = simulation.returnData?.decoded ?? null;

  if (decoded?.kind === "view_liquidation_price") {
    console.log(decoded.liquidationPriceTicks);
  }
  ```

  ```rust Rust theme={null}
  use phoenix_rise::ix::hawkeye::{
      HawkeyeReturnData,
      HawkeyeTraderViewAccounts,
      create_hawkeye_view_liquidation_price_ix,
  };

  let accounts = HawkeyeTraderViewAccounts {
      phoenix_program_id,
      global_config,
      global_trader_index,
      active_trader_buffer,
      perp_asset_map,
      trader: trader_account,
  };
  let ix = create_hawkeye_view_liquidation_price_ix(accounts, sol_asset_id);
  let simulation = hawkeye.simulate_instruction(ix.into()).await?;

  if let HawkeyeReturnData::LiquidationPrice(value) = simulation.value {
      println!("{}", value.liquidation_price_ticks);
  }
  ```
</CodeGroup>

Reference coverage: [ts/tests/sdk-localnet-flows.test.ts](https://github.com/Ellipsis-Labs/rise-public/blob/master/ts/tests/sdk-localnet-flows.test.ts)
executes all five Hawkeye views in LiteSVM, and
[rust/sdk/tests/sdk\_localnet\_fixture\_tests.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/sdk/tests/sdk_localnet_fixture_tests.rs)
asserts Hawkeye return data is emitted by the expected program id.

## Supported CPI Contexts

The typed CPI surface includes:

* Trader lifecycle: `RegisterTrader`, `SetTraderCapabilitiesDelegated`, `UpdateTraderState`, `SyncParentToChild`.
* Order flow: `PlaceMarketOrder`, `PlaceLimitOrder`, `PlaceMarketOrderDelegated`, `PlaceMultiLimitOrder`, `CancelAll`, `CancelUpTo`, `CancelOrdersById`, `CancelAllPlusConditional`.
* Collateral and subaccounts: `PhoenixDeposit`, `PhoenixWithdraw`, `TransferCollateral`, `TransferCollateralChildToParent`.
* Stop losses and conditionals: `CreateConditionalOrdersAccount`, `PlaceStopLoss`, `CancelStopLoss`, `PlacePositionConditionalOrder`, `PlaceAttachedConditionalOrder`, `PlaceLimitOrderWithConditionals`, `CancelConditionalOrder`.
* Ember collateral movement: `ember::EmberDeposit`, `ember::EmberWithdraw`.
* Hawkeye reads: `hawkeye::ViewMargin` / `hawkeye::HawkeyeViewMargin`.

For full account lists and data lengths, inspect
[rust/ix/src/cpi.rs](https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/ix/src/cpi.rs).

## Testing

Use [LiteSVM Testing](/sdk/litesvm-testing) for local integration tests. The
example program tests cover deposits, withdrawals, market orders, limit orders,
cancels, stop losses, Hawkeye reads, and isolated subaccount collateral flows.

<!-- END SOURCE: sdk/on-chain-programs.md -->

---

# Source: `sdk/orders.md`

**Original URL:** `https://docs.phoenix.trade/sdk/orders.md`

**Local path:** `sdk/orders.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Orders

> Build Phoenix order, cancellation, stop-loss, and conditional-order instructions with the Rise SDK.

Use this page when a trader account already exists and you want to build Solana instructions for trading. Your app still signs and sends the transaction with the correct authority, position authority, delegated wallet, or fee payer.

For account indexes, cross vs isolated accounts, and trader-state subscriptions across all subaccounts, see [Accounts](/sdk/accounts). For mark-price joins, account health, and liquidation estimates, see [Margin](/sdk/margin).

## Instruction Builders

All order builders produce Phoenix instructions. They do not submit transactions by themselves.

| Action                   | TypeScript builder                                   | Rust builder                                                               | Phoenix instruction             |
| ------------------------ | ---------------------------------------------------- | -------------------------------------------------------------------------- | ------------------------------- |
| Place limit order        | `client.ixs.buildPlaceLimitOrder(...)`               | `PhoenixTxBuilder::place_limit_order(...)`                                 | `PlaceLimitOrder`               |
| Cancel by order id       | `client.ixs.buildCancelOrdersById(...)`              | `PhoenixTxBuilder::build_cancel_orders(...)`                               | `CancelOrdersById`              |
| Cancel all in a market   | `client.ixs.buildCancelAll(...)`                     | `PhoenixTxBuilder::build_cancel_all_orders(...)`                           | `CancelAll`                     |
| Place market order       | `client.ixs.buildPlaceMarketOrder(...)`              | `PhoenixTxBuilder::place_market_order(...)`                                | `PlaceMarketOrder`              |
| Place position stop loss | `client.ixs.buildPlaceStopLoss(...)`                 | `PhoenixTxBuilder::build_stop_loss_orders(...)`                            | `PlaceStopLoss`                 |
| Place conditional order  | `client.ixs.buildPlacePositionConditionalOrder(...)` | `PhoenixTxBuilder::place_position_bracket_order(...)` or raw `ix` builders | `PlacePositionConditionalOrder` |

## Limit Order

<CodeGroup>
  ```ts TypeScript theme={null}
  import { Side } from "@ellipsis-labs/rise";

  await client.exchange.ready();

  const orderPacket = await client.orderPackets.buildLimitOrderPacket({
    symbol: "SOL",
    side: Side.Bid,
    priceUsd: "150.50",
    baseUnits: "0.25",
  });

  const ix = await client.ixs.buildPlaceLimitOrder({
    authority: "AUTHORITY_PUBKEY",
    symbol: "SOL",
    orderPacket,
    traderPdaIndex: 0,
    traderSubaccountIndex: 0,
  });
  ```

  ```rust Rust theme={null}
  use phoenix_rise::{LimitOrderTicket, PhoenixTxBuilder, Side, TraderKey};

  let trader = TraderKey::new(authority);
  let builder = PhoenixTxBuilder::new(&metadata);

  let ticket = LimitOrderTicket::builder()
      .authority(trader.authority())
      .trader_account(trader.pda())
      .symbol("SOL")
      .side(Side::Bid)
      .price(150.50)
      .num_base_lots(50_000)
      .build()?;

  let ixs = builder.place_limit_order(ticket).await?;
  ```
</CodeGroup>

References:

* [TypeScript limit-order example](https://github.com/Ellipsis-Labs/rise-public/blob/master/ts/examples/03-build-limit-order-ix.ts)
* [Rust limit-order example](https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/sdk/examples/send_limit_order.rs)

## Cancel Limit Order

<CodeGroup>
  ```ts TypeScript theme={null}
  const ix = await client.ixs.buildCancelOrdersById({
    authority: "AUTHORITY_PUBKEY",
    symbol: "SOL",
    orders: [
      {
        price: 50_000n,
        orderSequenceNumber: "12345",
      },
    ],
    traderPdaIndex: 0,
    traderSubaccountIndex: 0,
  });
  ```

  ```rust Rust theme={null}
  use phoenix_rise::ix::types::CancelId;

  let cancel_id = CancelId::new(50_000, 12_345);
  let ixs = builder.build_cancel_orders(
      trader.authority(),
      trader.pda(),
      "SOL",
      vec![cancel_id],
  )?;
  ```
</CodeGroup>

Reference: [Rust cancel-order example](https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/sdk/examples/cancel_order.rs).

## Cancel All

<CodeGroup>
  ```ts TypeScript theme={null}
  const ix = await client.ixs.buildCancelAll({
    authority: "AUTHORITY_PUBKEY",
    symbol: "SOL",
    traderPdaIndex: 0,
    traderSubaccountIndex: 0,
  });
  ```

  ```rust Rust theme={null}
  let ixs = builder.build_cancel_all_orders(
      trader.authority(),
      trader.pda(),
      "SOL",
  )?;
  ```
</CodeGroup>

## Market Order

<CodeGroup>
  ```ts TypeScript theme={null}
  import { Side } from "@ellipsis-labs/rise";

  const orderPacket = await client.orderPackets.buildMarketOrderPacket({
    symbol: "SOL",
    side: Side.Bid,
    baseUnits: "0.25",
  });

  const ix = await client.ixs.buildPlaceMarketOrder({
    authority: "AUTHORITY_PUBKEY",
    symbol: "SOL",
    orderPacket,
    traderPdaIndex: 0,
    traderSubaccountIndex: 0,
  });
  ```

  ```rust Rust theme={null}
  use phoenix_rise::{MarketOrderTicket, Side};

  let ticket = MarketOrderTicket::builder()
      .authority(trader.authority())
      .trader_account(trader.pda())
      .symbol("SOL")
      .side(Side::Bid)
      .num_base_lots(67)
      .build()?;

  let ixs = builder.place_market_order(ticket).await?;
  ```
</CodeGroup>

Reference: [Rust market-order example](https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/sdk/examples/send_market_order.rs).

## Stop Losses

Position stop losses use a per-trader, per-asset stop-loss account. If the account does not already exist, creating it can add an account-init instruction and incur rent.

<CodeGroup>
  ```ts TypeScript theme={null}
  import { Direction, Side, StopLossOrderKind } from "@ellipsis-labs/rise";

  const ix = await client.ixs.buildPlaceStopLoss({
    authority: "AUTHORITY_PUBKEY",
    symbol: "SOL",
    triggerPrice: 140_000n,
    slippageBps: 100,
    tradeSide: Side.Ask,
    executionDirection: Direction.LessThan,
    orderKind: StopLossOrderKind.IOC,
    traderPdaIndex: 0,
    traderSubaccountIndex: 0,
  });
  ```

  ```rust Rust theme={null}
  use phoenix_rise::{BracketLeg, BracketLegOrders, Side};

  let bracket = BracketLegOrders {
      stop_loss: Some(BracketLeg::new(140.0).with_slippage_bps(100)),
      take_profit: None,
  };

  let ixs = builder.build_stop_loss_orders(
      trader.authority(),
      trader.pda(),
      "SOL",
      Side::Bid,
      &bracket,
  )?;
  ```
</CodeGroup>

To cancel a position stop loss:

<CodeGroup>
  ```ts TypeScript theme={null}
  import { Direction } from "@ellipsis-labs/rise";

  const ix = await client.ixs.buildCancelStopLoss({
    authority: "AUTHORITY_PUBKEY",
    symbol: "SOL",
    executionDirection: Direction.LessThan,
    traderPdaIndex: 0,
    traderSubaccountIndex: 0,
  });
  ```

  ```rust Rust theme={null}
  use phoenix_rise::Direction;

  let ixs = builder.build_cancel_bracket_leg(
      trader.authority(),
      trader.pda(),
      "SOL",
      Direction::LessThan,
  )?;
  ```
</CodeGroup>

Reference: [Rust cancel stop-loss example](https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/sdk/examples/cancel_stop_loss.rs).

## Conditional Orders

Conditional orders use a trader conditional-orders account. If the account does not already exist, your flow may need to create it first and pay rent. The newer bracket helpers can check and create the account when configured with an RPC client; raw builders require you to include `CreateConditionalOrdersAccount` yourself when needed.

<CodeGroup>
  ```ts TypeScript theme={null}
  import {
    Direction,
    Side,
    StopLossOrderKind,
    ticks,
  } from "@ellipsis-labs/rise";

  declare const conditionalOrdersAccountExists: boolean;

  const ixs = [];

  if (!conditionalOrdersAccountExists) {
    ixs.push(
      await client.ixs.buildCreateConditionalOrdersAccount({
        authority: "AUTHORITY_PUBKEY",
        traderPdaIndex: 0,
        traderSubaccountIndex: 0,
      })
    );
  }

  ixs.push(
    await client.ixs.buildPlacePositionConditionalOrder({
      authority: "AUTHORITY_PUBKEY",
      symbol: "SOL",
      lessTriggerOrder: {
        triggerDirection: Direction.LessThan,
        tradeSide: Side.Ask,
        orderKind: StopLossOrderKind.IOC,
        triggerPrice: ticks(140_000n),
        slippageBps: 100,
      },
      sizePercent: 100,
      traderPdaIndex: 0,
      traderSubaccountIndex: 0,
    })
  );
  ```

  ```rust Rust theme={null}
  use std::sync::Arc;
  use phoenix_rise::{
      BracketLeg, BracketLegOrders, BracketLegTicket, Side,
  };
  use solana_rpc_client::nonblocking::rpc_client::RpcClient;

  let rpc = Arc::new(RpcClient::new("https://api.mainnet-beta.solana.com".to_string()));
  let bracket = BracketLegOrders {
      stop_loss: Some(BracketLeg::new(140.0).with_slippage_bps(100)),
      take_profit: Some(BracketLeg::new(180.0).with_limit_order()),
  };

  let ixs = builder
      .place_position_bracket_order(
          trader.authority(),
          trader.pda(),
          "SOL",
          Side::Bid,
          BracketLegTicket::new(rpc, bracket),
      )
      .await?;
  ```
</CodeGroup>

For bulk cancellation, read the conditional-orders account, enumerate active trigger legs, and build one cancel instruction per active leg. The public examples show the full account-read and batching flow:

* [TypeScript cancel all conditional orders](https://github.com/Ellipsis-Labs/rise-public/blob/master/ts/examples/05-cancel-all-conditional-orders.ts)
* [Rust cancel all conditional orders](https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/sdk/examples/cancel_all_conditional_orders.rs)

<!-- END SOURCE: sdk/orders.md -->

---

# Source: `sdk/register.md`

**Original URL:** `https://docs.phoenix.trade/sdk/register.md`

**Local path:** `sdk/register.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Trader Onboarding

> Activate Phoenix trader accounts with or without a referral code.

<Warning>
  Do not build new integrations around `/v1/invite/activate` or `/v1/referral/activate`. These legacy sponsored activation routes are deprecated; use `/v1/referral/activate-tx` for referral-code onboarding.
</Warning>

Most users should onboard in the Phoenix app at [phoenix.trade](https://phoenix.trade). Use the SDK flows below when you are building onboarding into your own app.

There are two SDK onboarding paths:

1. With a referral code: call `POST /v1/referral/activate-tx`. This path requires auth and the user-controlled trader authority signature.
2. Without a referral code: call `POST /v1/exchange/build-register-ixs`, build and sign the transaction locally, then submit it to `POST /v1/exchange/send-register-ixs`. This path does not validate the trader authority signature and can register off-curve authorities such as PDAs.

Signer handling differs by path. For referral-code activation, the request must include an authenticated Phoenix session, the user authority must sign the transaction request, and the API adds the Phoenix referral onboarder signature before submitting it. For the no-referral builder flow, `traderAuthority` is only the pubkey used to derive the trader account; it is not required to be a signer. The transaction sent to `send-register-ixs` must be signed by the fee payer and any other signer accounts you add. In the no-referral builder flow, `pda_index` and `subaccount_index` must be `0`; `max_positions` is optional and defaults to `128`.

<Note>
  Trader onboarding registers the default cross-margin account at `traderPdaIndex = 0` and `traderSubaccountIndex = 0`. Its `max_positions` value must be between `32` and `128` inclusive. Omit the field to use the default `128`.
</Note>

## With a referral code

Use `/v1/referral/activate-tx` when the user has a valid referral code. The TypeScript SDK builds the transaction, checks whether the default trader account needs to be registered, and submits the signed transaction request to the API.

If your app pays for onboarding, pass `feePayer` and sign the transaction with both the payer and the trader authority. If the trader pays, omit `feePayer`.

<CodeGroup>
  ```ts TypeScript theme={null}
  import { createPhoenixClient, type Authority } from "@ellipsis-labs/rise";
  import { createKeyPairSignerFromBytes } from "@solana/signers";
  import {
    createSolanaRpc,
    partiallySignTransaction,
  } from "@solana/kit";

  declare const traderKeypairBytes: Uint8Array;

  const client = createPhoenixClient({
    apiUrl: "https://perp-api.phoenix.trade",
    rpcUrl: "https://api.mainnet-beta.solana.com",
    exchangeMetadata: { stream: false },
  });
  const rpc = createSolanaRpc("https://api.mainnet-beta.solana.com");
  const traderSigner = await createKeyPairSignerFromBytes(traderKeypairBytes);
  const traderAuthority = traderSigner.address as Authority;
  const latestBlockhash = await rpc
    .getLatestBlockhash({ commitment: "finalized" })
    .send();

  const built = await client.api.invite().buildActivateReferralTxRequest({
    referralCode: "REFERRAL_CODE",
    traderAuthority,
    traderPdaIndex: 0,
    traderSubaccountIndex: 0,
    recentBlockhash: latestBlockhash.value.blockhash,
    lastValidBlockHeight: latestBlockhash.value.lastValidBlockHeight,
    registerTraderMaxPositions: 128n,
    rpc,
    signTransaction: (transaction) =>
      partiallySignTransaction([traderSigner.keyPair], transaction),
  });

  const response = await client.api.invite().activateReferralTx(built.request);
  console.log(response.signature, response.trader_pda, response.status);
  ```

  ```rust Rust theme={null}
  use base64::Engine as _;
  use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
  use phoenix_rise::{
      ActivateReferralTxRequest, PhoenixHttpClient, PhoenixMetadata, TraderKey,
      fetch_referral_activation_trader_status,
  };
  use phoenix_rise::phoenix_rise_ix::{
      OnboardTraderDelegatedParams, RegisterTraderParams, create_onboard_trader_delegated_ix,
      create_register_trader_ix,
  };
  use solana_commitment_config::CommitmentConfig;
  use solana_instruction::Instruction;
  use solana_keypair::read_keypair_file;
  use solana_rpc_client::nonblocking::rpc_client::RpcClient;
  use solana_signer::Signer;
  use solana_transaction::Transaction;
  use solana_transaction::versioned::VersionedTransaction;

  let http = PhoenixHttpClient::new_public("https://perp-api.phoenix.trade")?;
  let rpc = RpcClient::new_with_commitment(
      "https://api.mainnet-beta.solana.com".to_string(),
      CommitmentConfig::confirmed(),
  );
  let trader_keypair = read_keypair_file("/path/to/trader-keypair.json")?;
  let trader = TraderKey::new_with_idx(trader_keypair.pubkey(), 0, 0);
  let permission = http.invite().get_referral_activation_permission().await?;
  let exchange = PhoenixMetadata::new(http.get_exchange().await?.into());
  let status = fetch_referral_activation_trader_status(&rpc, &trader.pda()).await?;

  let mut ixs = Vec::<Instruction>::new();
  if status.should_include_register_trader() {
      ixs.push(
          create_register_trader_ix(
              RegisterTraderParams::builder()
                  .payer(trader_keypair.pubkey())
                  .trader(trader.authority())
                  .trader_account(trader.pda())
                  .max_positions(128)
                  .trader_pda_index(0)
                  .subaccount_index(0)
                  .build()?,
          )?
          .into(),
      );
  }
  ixs.push(
      create_onboard_trader_delegated_ix(
          OnboardTraderDelegatedParams::builder()
              .authority(permission.trader_onboarder.parse()?)
              .permission_account(permission.permission_account.parse()?)
              .trader_account(trader.pda())
              .global_trader_index(
                  exchange.keys().global_trader_index
                      .iter()
                      .map(|key| key.parse())
                      .collect::<Result<Vec<_>, _>>()?,
              )
              .active_trader_buffer(
                  exchange.keys().active_trader_buffer
                      .iter()
                      .map(|key| key.parse())
                      .collect::<Result<Vec<_>, _>>()?,
              )
              .build()?,
      )?
      .into(),
  );

  let recent_blockhash = rpc.get_latest_blockhash().await?;
  let mut tx = Transaction::new_with_payer(&ixs, Some(&trader_keypair.pubkey()));
  tx.try_partial_sign(&[&trader_keypair], recent_blockhash)?;
  let transaction = BASE64_STANDARD.encode(bincode::serialize(
      &VersionedTransaction::from(tx),
  )?);

  let response = http.invite().activate_referral_tx(&ActivateReferralTxRequest {
      referral_code: "REFERRAL_CODE".to_string(),
      trader_authority: trader.authority().to_string(),
      trader_pda_index: Some(0),
      trader_subaccount_index: Some(0),
      recent_blockhash: recent_blockhash.to_string(),
      transaction,
  }).await?;
  println!("{response:?}");
  ```
</CodeGroup>

Full runnable examples:

* [TypeScript referral activation](https://github.com/Ellipsis-Labs/rise-public/blob/master/ts/examples/09-referral-activation-tx.ts)
* [Rust referral activation](https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/sdk/examples/referral_activation_tx.rs)

## Without a referral code

Use this path when a builder wants to register and onboard a trader without a referral code.

The flow is:

1. Call `POST /v1/exchange/build-register-ixs` with the trader authority pubkey and the wallet that will pay transaction fees and account rent.
2. Build a Solana transaction locally with those returned instructions, your current blockhash, and your chosen fee payer.
3. Sign the transaction with the fee payer and any other signer accounts you add.
4. Send the base64-encoded wire transaction to `POST /v1/exchange/send-register-ixs`.
5. The API validates the transaction, signs with the Phoenix onboarder, simulates it, verifies the onboarder pays no lamports, sends it to Phoenix's RPC, and returns the transaction signature.

The no-referral path does not validate that `traderAuthority` can sign. You may pass an on-curve wallet address or an off-curve PDA as the trader authority.

The fee payer must not be the Phoenix onboarder key. Your app should await confirmation for the returned signature.

<CodeGroup>
  ```ts TypeScript theme={null}
  import {
    createPhoenixClient,
    type Authority,
    type RegisterIxInstruction,
  } from "@ellipsis-labs/rise";
  import { createKeyPairSignerFromBytes } from "@solana/signers";
  import {
    AccountRole,
    address,
    appendTransactionMessageInstructions,
    compileTransaction,
    createSolanaRpc,
    createTransactionMessage,
    getBase64EncodedWireTransaction,
    partiallySignTransaction,
    pipe,
    setTransactionMessageFeePayer,
    setTransactionMessageLifetimeUsingBlockhash,
    type Blockhash,
  } from "@solana/kit";

  declare const traderAuthority: Authority;
  declare const feePayerKeypairBytes: Uint8Array;

  const toInstruction = (ix: RegisterIxInstruction) => ({
    programAddress: address(ix.programId),
    accounts: ix.keys.map((account) => ({
      address: address(account.pubkey),
      role: account.isSigner
        ? account.isWritable
          ? AccountRole.WRITABLE_SIGNER
          : AccountRole.READONLY_SIGNER
        : account.isWritable
          ? AccountRole.WRITABLE
          : AccountRole.READONLY,
    })),
    data: Uint8Array.from(ix.data),
  });

  const client = createPhoenixClient({
    apiUrl: "https://perp-api.phoenix.trade",
    rpcUrl: "https://api.mainnet-beta.solana.com",
    exchangeMetadata: { stream: false },
  });
  const rpc = createSolanaRpc("https://api.mainnet-beta.solana.com");
  const feePayerSigner = await createKeyPairSignerFromBytes(feePayerKeypairBytes);
  const txFeePayer = feePayerSigner.address as Authority;
  const latestBlockhash = await rpc
    .getLatestBlockhash({ commitment: "finalized" })
    .send();

  const built = await client.api.exchange().buildRegisterIxs({
    traderAuthority,
    txFeePayer,
    maxPositions: 128,
  });

  const message = pipe(
    createTransactionMessage({ version: 0 }),
    (tx) => setTransactionMessageFeePayer(txFeePayer, tx),
    (tx) =>
      setTransactionMessageLifetimeUsingBlockhash(
        {
          blockhash: latestBlockhash.value.blockhash as Blockhash,
          lastValidBlockHeight: latestBlockhash.value.lastValidBlockHeight,
        },
        tx,
      ),
    (tx) => appendTransactionMessageInstructions(
      built.instructions.map(toInstruction),
      tx,
    ),
  );

  const signed = await partiallySignTransaction(
    [feePayerSigner.keyPair],
    compileTransaction(message),
  );

  const response = await client.api.exchange().sendRegisterIxs({
    transaction: getBase64EncodedWireTransaction(signed),
    traderAuthority,
    txFeePayer,
    maxPositions: 128,
    traderPdaIndex: 0,
    traderSubaccountIndex: 0,
  });

  console.log(response.signature, response.traderPda);
  ```

  ```rust Rust theme={null}
  use base64::Engine as _;
  use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
  use phoenix_rise::{
      ApiInstructionResponse, BuildRegisterIxsRequest, PhoenixHttpClient, SendRegisterIxsRequest,
  };
  use solana_commitment_config::CommitmentConfig;
  use solana_instruction::{AccountMeta, Instruction};
  use solana_keypair::read_keypair_file;
  use solana_pubkey::Pubkey;
  use solana_rpc_client::nonblocking::rpc_client::RpcClient;
  use solana_signer::Signer;
  use solana_transaction::Transaction;
  use solana_transaction::versioned::VersionedTransaction;

  fn to_instruction(ix: &ApiInstructionResponse) -> Result<Instruction, Box<dyn std::error::Error>> {
      Ok(Instruction {
          program_id: ix.program_id.parse()?,
          accounts: ix.keys.iter().map(|account| {
              Ok(AccountMeta {
                  pubkey: account.pubkey.parse()?,
                  is_signer: account.is_signer,
                  is_writable: account.is_writable,
              })
          }).collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?,
          data: ix.data.clone(),
      })
  }

  let http = PhoenixHttpClient::new_public("https://perp-api.phoenix.trade")?;
  let rpc = RpcClient::new_with_commitment(
      "https://api.mainnet-beta.solana.com".to_string(),
      CommitmentConfig::confirmed(),
  );
  let fee_payer_keypair = read_keypair_file("/path/to/fee-payer-keypair.json")?;
  let trader_authority = "PDA_OR_AUTHORITY_PUBKEY".parse::<Pubkey>()?;
  let tx_fee_payer = fee_payer_keypair.pubkey();

  let built = http.exchange().build_register_ixs(&BuildRegisterIxsRequest {
      trader_authority: trader_authority.to_string(),
      tx_fee_payer: tx_fee_payer.to_string(),
      max_positions: Some(128),
  }).await?;
  let ixs = built.instructions
      .iter()
      .map(to_instruction)
      .collect::<Result<Vec<_>, _>>()?;

  let recent_blockhash = rpc.get_latest_blockhash().await?;
  let mut tx = Transaction::new_with_payer(&ixs, Some(&tx_fee_payer));
  tx.try_partial_sign(&[&fee_payer_keypair], recent_blockhash)?;
  let transaction = BASE64_STANDARD.encode(bincode::serialize(
      &VersionedTransaction::from(tx),
  )?);

  let response = http.exchange().send_register_ixs(&SendRegisterIxsRequest {
      transaction,
      trader_authority: trader_authority.to_string(),
      tx_fee_payer: tx_fee_payer.to_string(),
      max_positions: Some(128),
      trader_pda_index: Some(0),
      trader_subaccount_index: Some(0),
  }).await?;
  println!("submitted {}", response.signature);
  ```
</CodeGroup>

Full runnable examples:

* [TypeScript builder onboarding transaction](https://github.com/Ellipsis-Labs/rise-public/blob/master/ts/examples/10-builder-onboarding-tx.ts)
* [Rust builder onboarding transaction](https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/sdk/examples/builder_onboarding_tx.rs)

## After onboarding

Both paths activate the trader's default cross-margin account at `traderPdaIndex = 0` and `traderSubaccountIndex = 0`. The cross account's `max_positions` value is set during registration and must be between `32` and `128` inclusive.

Next steps:

* Learn account indexes, isolated accounts, position authority, and trader-state subscriptions in [Accounts](/sdk/accounts).
* Fund the account in [Collateral](/sdk/collateral).
* Send orders in [Orders](/sdk/orders) and monitor account health in [Margin](/sdk/margin).

<!-- END SOURCE: sdk/register.md -->

---

# Source: `sdk/rise.md`

**Original URL:** `https://docs.phoenix.trade/sdk/rise.md`

**Local path:** `sdk/rise.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# Rise SDK

> Developer SDKs for Phoenix perpetuals in TypeScript and Rust.

Rise is the developer-facing SDK surface for Phoenix perpetual futures. Use it after you understand the protocol concepts in the Phoenix docs and want to build an app, service, bot, or on-chain integration that reads Phoenix state and sends Phoenix instructions.

The SDK sources live in the public repo: [github.com/Ellipsis-Labs/rise-public](https://github.com/Ellipsis-Labs/rise-public).

The SDK currently ships as:

* `rise/ts` - TypeScript SDK with HTTP route clients, a unified Phoenix client, exchange metadata caching, instruction builders, order-packet helpers, WebSocket adapters, trader-state stores, and margin helpers.
* `rise/rust` - Rust workspace centered on the `phoenix-rise` crate, with typed HTTP and WebSocket clients, `PhoenixMetadata`, `PhoenixTxBuilder`, trader-state containers, margin math, and low-level instruction builders.

## Install

### TypeScript

<CodeGroup>
  ```bash npm theme={null}
  npm add @ellipsis-labs/rise
  ```

  ```bash bun theme={null}
  bun add @ellipsis-labs/rise
  ```

  ```bash pnpm theme={null}
  pnpm add @ellipsis-labs/rise
  ```
</CodeGroup>

### Rust

<CodeGroup>
  ```bash Default SDK theme={null}
  cargo add phoenix-rise
  ```

  ```bash API only theme={null}
  cargo add phoenix-rise --no-default-features --features api
  ```

  ```bash On-chain CPI theme={null}
  cargo add phoenix-rise --no-default-features --features cpi
  ```
</CodeGroup>

## Create A Client

<CodeGroup>
  ```ts TypeScript theme={null}
  import { createPhoenixClient } from "@ellipsis-labs/rise";

  const client = createPhoenixClient({
    apiUrl: "https://perp-api.phoenix.trade",
    rpcUrl: "https://api.mainnet-beta.solana.com",
    ws: { connectMode: "eager" },
    exchangeMetadata: { stream: true },
  });
  ```

  ```rust Rust theme={null}
  use phoenix_rise::{PhoenixHttpClient, PhoenixMetadata};

  let http = PhoenixHttpClient::new_from_env()?;
  let exchange = http.get_exchange().await?.into();
  let metadata = PhoenixMetadata::new(exchange);
  ```
</CodeGroup>

For package setup and runnable examples, use the public SDK sources:

* [TypeScript README](https://github.com/Ellipsis-Labs/rise-public/blob/master/ts/README.md)
* [TypeScript examples](https://github.com/Ellipsis-Labs/rise-public/tree/master/ts/examples)
* [Rust README](https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/README.md)
* [Rust SDK examples](https://github.com/Ellipsis-Labs/rise-public/tree/master/rust/sdk/examples)

## Which Client To Use

| Task                                                                   | TypeScript                                                      | Rust                                    |
| ---------------------------------------------------------------------- | --------------------------------------------------------------- | --------------------------------------- |
| HTTP snapshots, history, and transaction-builder routes                | `client.api` or `PhoenixHttpClient`                             | `PhoenixHttpClient`                     |
| Exchange params, market metadata, PDA context, and order-packet sizing | `client.exchange` cache                                         | `PhoenixMetadata`                       |
| Live market, orderbook, exchange, and trader streams                   | `client.streams`, `client.marketData()`, `client.orderbooks()`  | `PhoenixWSClient` plus typed containers |
| Trader snapshots and deltas                                            | `createTraderStateStore(client)`                                | `Trader::apply_update(...)`             |
| Build Phoenix instructions                                             | `client.ixs`                                                    | `PhoenixTxBuilder`                      |
| Margin and account-health calculations                                 | `createMarginCalculator(...)` and trader-state `marginInputs()` | `TraderPortfolio::compute_margin(...)`  |

Prefer the exchange cache for market parameters. In TypeScript, wait for `client.exchange.ready()` and read markets with `client.exchange.market(symbol)` or `client.exchange.marketMetadata(symbol)`. In Rust, build `PhoenixMetadata` from `http.get_exchange()` and update it with live market stats through `metadata.apply_market_stats(&stats)`.

Prefer trader-state primitives for snapshots and deltas. In TypeScript, `createTraderStateStore(client)` maintains local trader resources and exposes `marginInputs()`. In Rust, `Trader::apply_update(&msg)` applies `traderState` snapshots and deltas to the local container.

## First Reads

<CodeGroup>
  ```ts TypeScript theme={null}
  const snapshot = await client.exchange.ready();
  const sol = client.exchange.market("SOL");
  const solMetadata = client.exchange.marketMetadata("SOL");
  const trader = await client.api
    .traders()
    .getTraderStateSnapshot("AUTHORITY_PUBKEY", { traderPdaIndex: 0 });

  console.log(snapshot.markets.length, sol?.assetId, solMetadata?.logoUri);
  console.log(trader.snapshot.subaccounts);
  ```

  ```rust Rust theme={null}
  use phoenix_rise::{PhoenixHttpClient, PhoenixMetadata};
  use solana_pubkey::Pubkey;
  use std::str::FromStr;

  let http = PhoenixHttpClient::new_from_env()?;
  let exchange = http.get_exchange().await?.into();
  let metadata = PhoenixMetadata::new(exchange);
  let authority = Pubkey::from_str("AUTHORITY_PUBKEY")?;

  let sol = metadata.get_market("SOL");
  let trader = http.traders().get_trader(&authority).await?;

  println!("{:?} {}", sol.map(|m| &m.symbol), trader.len());
  ```
</CodeGroup>

Runnable references:

* [TypeScript HTTP client example](https://github.com/Ellipsis-Labs/rise-public/blob/master/ts/examples/01-http-client.ts)
* [TypeScript unified client example](https://github.com/Ellipsis-Labs/rise-public/blob/master/ts/examples/phoenix-client-example.ts)
* [Rust HTTP client example](https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/sdk/examples/http_client.rs)
* [Rust Phoenix client example](https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/sdk/examples/phoenix_client.rs)

## Reading Order

1. [Auth](/sdk/auth) - optional for most public reads, required for referral onboarding and notifications.
2. [Trader Onboarding](/sdk/register) - create or activate a trader account.
3. [Accounts](/sdk/accounts) - understand authority, position authority, cross accounts, isolated accounts, and trader-state streams.
4. [Collateral](/sdk/collateral) - move USDC into and out of Phoenix trader accounts.
5. [Exchange Data](/sdk/markets) - exchange cache, market metadata, L2 orderbooks, market stats, and market calendars.
6. [WS/API Best Practices](/sdk/ws-api-best-practices) - when to stream, when to poll, and how to avoid rate-limit and drift issues.
7. [Orders](/sdk/orders) - build order instructions and manage stop losses and conditionals.
8. [Margin](/sdk/margin) - join market prices, compute account health, margin, and liquidation estimates.
9. [On-Chain Programs](/sdk/on-chain-programs) and [LiteSVM Testing](/sdk/litesvm-testing) - program integrations and local tests.

## REST, WebSocket, and RPC

Use REST for startup snapshots, historical data, and server-built transaction workflows. Use WebSocket streams for current market state, exchange deltas, orderbooks, and trader state. Use Solana RPC for blockhashes, simulation, transaction submission, confirmation, and custom account reads.

The usual production pattern is:

1. Fetch HTTP snapshots on startup.
2. Subscribe to WebSocket streams.
3. Apply exchange and trader deltas locally through SDK caches.
4. Build and sign instructions with the user's wallet or signer service.
5. Confirm transactions through Solana RPC.
6. Reconcile from trader-state streams, falling back to fresh HTTP snapshots after reconnects or sequence gaps.

<!-- END SOURCE: sdk/rise.md -->

---

# Source: `sdk/ws-api-best-practices.md`

**Original URL:** `https://docs.phoenix.trade/sdk/ws-api-best-practices.md`

**Local path:** `sdk/ws-api-best-practices.md`

---

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.phoenix.trade/llms.txt
> Use this file to discover all available pages before exploring further.

# WS/API Best Practices

> Use Phoenix REST, WebSocket, and RPC surfaces without drifting state or hitting rate limits.

Use REST for snapshots, historical reads, and server-built transaction workflows. Use WebSockets for live exchange, market, orderbook, and trader-state updates. Use Solana RPC for blockhashes, simulation, transaction submission, confirmation, and custom account reads.

## Default Pattern

1. Fetch startup snapshots over REST.
2. Start WebSocket subscriptions for exchange metadata, market stats, orderbooks, and trader state.
3. Apply snapshots and deltas through SDK primitives.
4. Build instructions from the exchange cache.
5. Sign and send through the user's wallet or signer service.
6. Reconcile from trader-state streams; refetch REST snapshots after reconnects or sequence gaps.

<CodeGroup>
  ```ts TypeScript theme={null}
  import { createPhoenixClient, createTraderStateStore } from "@ellipsis-labs/rise";

  const client = createPhoenixClient({
    apiUrl: "https://perp-api.phoenix.trade",
    rpcUrl: "https://api.mainnet-beta.solana.com",
    ws: { connectMode: "eager" },
    exchangeMetadata: { stream: true },
  });

  await client.exchange.ready();

  const traderState = createTraderStateStore(client);
  const trader = traderState.resource({
    authority: "AUTHORITY_PUBKEY",
    traderPdaIndex: 0,
  });

  const release = trader.retain();
  await trader.ready();
  ```

  ```rust Rust theme={null}
  use phoenix_rise::api::{PhoenixHttpClient, PhoenixMetadata, PhoenixWSClient, Trader, TraderKey};
  use solana_pubkey::Pubkey;

  let http = PhoenixHttpClient::new_from_env()?;
  let exchange = http.get_exchange().await?.into();
  let mut metadata = PhoenixMetadata::new(exchange);

  let authority: Pubkey = "AUTHORITY_PUBKEY".parse()?;
  let trader_key = TraderKey::from_authority(authority);
  let mut trader = Trader::new(trader_key.clone());

  let ws = PhoenixWSClient::new_from_env()?;
  let (mut trader_rx, _handle) = ws.subscribe_to_trader_state(&authority)?;
  ```
</CodeGroup>

## Rate Limits

Most public market and trader reads do not require auth, but authenticated sessions receive higher API limits. Auth is required for referral-code onboarding and notifications.

To avoid rate-limit pressure:

* Prefer WebSocket streams for frequently changing state.
* Share one process-level Phoenix client where possible.
* Share one subscription per market or trader and fan out locally.
* Cache exchange params with `client.exchange` or `PhoenixMetadata`.
* Cache trader state with `createTraderStateStore(client)` or `Trader::apply_update(...)`.
* Use REST polling for slow reconciliation, not frame-by-frame UI updates.
* Back off on `429` responses and retry after the server-provided delay when available.

## Snapshots And Deltas

Do not hand-roll snapshot/delta application unless you are building SDK internals.

In TypeScript:

* `client.exchange` bootstraps from API or RPC, applies exchange WebSocket snapshots and deltas, and emits cache events.
* `createTraderStateStore(client)` applies trader-state snapshots and deltas and exposes derived positions, orders, triggers, history, and `marginInputs()`.
* `client.marketData()` combines all-mids, market-stats, and mark-price streams into per-symbol rows.

In Rust:

* `PhoenixMetadata::new(exchange)` creates the exchange cache from an HTTP snapshot.
* `metadata.apply_market_stats(&stats)` updates market mark-price inputs used by margin math.
* `Trader::apply_update(&msg)` applies trader-state WebSocket updates to a local trader container.
* `L2Book::apply_update(&msg)` maintains an orderbook container.

## Reconnects

WebSocket clients can reconnect, but your application still needs a reconciliation policy. After a reconnect, sequence gap, or prolonged disconnect:

* Refresh exchange metadata over REST or wait for a fresh exchange snapshot.
* Refetch or await a fresh trader-state snapshot before showing account health as final.
* Drop stale L2 book state unless the SDK resource confirms it has been resynced.
* Recompute margin only after both trader state and market prices are current.

## Notifications

Notifications are an authenticated workflow. Use a Phoenix auth session when subscribing to user-specific notifications over REST or WebSocket. Public market streams should stay unauthenticated unless your integration needs authenticated API limits for the rest of its traffic.

References:

* [Auth](/sdk/auth)
* [Exchange Data](/sdk/markets)
* [Accounts](/sdk/accounts)
* [Orders](/sdk/orders)
* [Margin](/sdk/margin)
* [WebSocket API](/api/websocket)
* [TypeScript WebSocket example](https://github.com/Ellipsis-Labs/rise-public/blob/master/ts/examples/phoenix-ws-example.ts)
* [Rust market-stats example](https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/sdk/examples/subscribe_market_stats.rs)

<!-- END SOURCE: sdk/ws-api-best-practices.md -->

---

