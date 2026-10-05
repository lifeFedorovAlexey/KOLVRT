# Linux allocation context research

Document status: CURRENT
Evidence scope: documentation review of Linux v6.12 pinned at adc218676eef25575469234709c2d87185ca223a; no reproduced driver execution, first-origin history or KOLVRT allocation service.
Current reference: [Existing candidate record](../cost-l/COST-L-0001.json)

<a name="linux-memory-allocation"></a>

## Allocation context and reclaim

The primary source distinguishes GFP_KERNEL sleeping/direct reclaim and context restrictions. This motivates studying recursive reclaim and callback/lifetime dependencies; lexical allocation references do not establish a driver's complete configured dependency closure. Historical introduction remains UNKNOWN in the existing candidate record.

KOLVRT comparison: current IRQ paths do not allocate or take heap/table locks. A future EL0 allocation service must explicitly define reclaim/failure and native authorization. The candidate's native disposition remains PROPOSED; no new accepted architecture or Linux driver support is established. COST-L-0001 remains CANDIDATE, not an active compatibility implementation.

Source: [Linux v6.12 memory-allocation.rst](https://github.com/torvalds/linux/blob/adc218676eef25575469234709c2d87185ca223a/Documentation/core-api/memory-allocation.rst), path Documentation/core-api/memory-allocation.rst, locator “Get Free Page flags”, reviewed 2026-10-04. Scope is generic documented allocation policy; architecture/configuration-specific driver behavior was not executed. Follow the mechanism template for future lifecycle, concurrency, failure, performance and historical investigation. No measured KOLVRT cost is known.

Open questions: original introducing commits; representative configured driver consumers; recurring semantic need; exact native-service authority/lifetime; measured adapter-versus-native costs. Coordinate existing COST-L research rather than automatically confirming this candidate.

[Russian translation](../../translations/ru/research/linux/memory-allocation.md)

<!-- knowledge -->

```json
{
  "schema_version": 1,
  "id": "doc.linux.memory.allocation",
  "kind": "linux-mechanism",
  "summary": "Pinned allocation-context and reclaim research; native service remains proposed.",
  "units": [
    {
      "id": "linux.memory.allocation",
      "anchor": "linux-memory-allocation",
      "kind": "linux-mechanism",
      "summary": "Linux v6.12 allocation context; no configured driver or native service execution.",
      "tags": ["linux", "memory", "allocation", "reclaim"],
      "relationships": [
        {
          "type": "motivates",
          "to": "cost-l.0001",
          "confidence": "HYPOTHESIS",
          "scope": "Research candidate, not confirmed driver dependency.",
          "evidence": ["research/cost-l/COST-L-0001.json"]
        }
      ]
    }
  ]
}
```
