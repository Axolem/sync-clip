# Encrypted relay for Clip delivery

Devices need to exchange Clips across networks without accounts or plaintext on infrastructure. v1 uses an encrypted relay: Shells connect using material derived from the Link Key; the relay only stores/forwards ciphertext. A first-party hosted relay is the default; Shells may point at a self-hosted relay instead. Direct P2P and LAN-only discovery are deferred so phone↔laptop works across NAT/cellular without making transport the product.
