---
title: "BlockPlace: Proof of Existence for Photos Using Blockchain and 3D Visualization"
date: 2026-02-24
tags: [Blockchain, Ethereum, Three.js, AI, Web3]
description: "A technical deep-dive into BlockPlace, a Web3 app that records SHA-256 photo hashes on Ethereum and converts images into 3D point clouds using AI depth estimation."
---

## Introduction

How do we guarantee the authenticity of digital photos? In an era flooded with fake images and AI-generated content, proving "when and by whom a photo was created" has become a critical challenge.

BlockPlace addresses this challenge with a blockchain-based solution. It records the SHA-256 hash of a photo on Ethereum (Sepolia), creating a tamper-proof, timestamped proof of existence. On top of that, AI depth estimation converts the photo into a 3D point cloud, delivering a visually compelling experience.

## System Architecture

BlockPlace is built on the following technology stack:

- **Frontend**: Next.js 16 + React 19 + TypeScript
- **3D Rendering**: Three.js + React Three Fiber
- **Blockchain**: Ethers.js 6 + Solidity
- **AI Depth Estimation**: Hugging Face Transformers (Depth Anything V2)
- **Local DB**: Dexie (IndexedDB)

The overall application flow works as follows:

1. The user uploads a photo
2. The SHA-256 hash is computed client-side
3. The AI depth estimation model generates a depth map
4. A 3D point cloud is constructed from the depth map
5. The hash is registered on the smart contract via MetaMask
6. Anyone can verify the hash on-chain

## How Blockchain Proof Works

### Smart Contract Design

BlockPlace's Solidity contract follows a simple yet robust design:

- **One-time registration**: The same hash can never be registered twice
- **Timestamp**: Records block creation time via `block.timestamp`
- **Owner tracking**: Stores the registrant's address
- **Revocation**: Only the owner can invalidate a proof

### Client-Side Hash Computation

Hash computation, the cornerstone of tamper prevention, runs in the browser using the Web Crypto API. By including both photo order and metadata in the hash, it guarantees content integrity.

## Leveraging AI Depth Estimation

### Asynchronous Processing with Web Workers

Depth estimation runs the Depth Anything V2 model in the browser using Hugging Face's Transformers.js. It leverages GPU acceleration via WebGPU when available, falling back to WASM otherwise.

### Optimizing Pipeline Processing

When processing multiple photos, efficiency is improved by parallelizing image preprocessing and depth estimation.

## 3D Visualization Implementation

### Converting Depth Maps to Point Clouds

Using camera intrinsic parameters (focal length, optical center), 2D images combined with depth information are back-projected into 3D spatial coordinates. The `downsample` parameter adjusts point density, balancing performance and visual quality.

### Rendering with React Three Fiber

The generated point cloud is built as a Three.js `BufferGeometry` and rendered with React Three Fiber. `OrbitControls` enables mouse-driven rotation and zoom, allowing users to freely explore the 3D space.

## Key Technical Challenges and Solutions

### 1. Running Large Models in the Browser

The Depth Anything V2 model weighs in at tens of megabytes. By leveraging the browser's Cache API to cache the model, we significantly sped up loading on subsequent visits.

### 2. Gas Fee Visualization

To help users understand costs before submitting a transaction, we combine Ethers.js's `estimateGas` and `getFeeData` to pre-calculate gas fees and display them in the UI.

### 3. Offline Support

Capture data is stored locally using IndexedDB (Dexie). Users can browse photos and view 3D visualizations offline, then register them on the blockchain once they are back online.

## Future Outlook

- **Multi-chain support**: Reducing gas fees by supporting Layer 2 networks such as Polygon and Arbitrum
- **IPFS integration**: Storing photos on IPFS and recording metadata URIs on-chain
- **Certificate issuance**: Exporting proof-of-existence certificates in PDF or NFT format
- **Batch registration**: Efficiently registering multiple captures in a single transaction

BlockPlace combines the immutability of blockchain with the potential of AI technology to tackle the modern challenge of digital content authenticity.
