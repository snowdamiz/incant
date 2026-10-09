<script setup lang="ts">
import SiteHeader from './components/SiteHeader.vue'
import HeroSection from './components/HeroSection.vue'
import FeatureSection from './components/FeatureSection.vue'
import WorkflowSection from './components/WorkflowSection.vue'
import TrustSection from './components/TrustSection.vue'
import FaqSection from './components/FaqSection.vue'
import ClosingSection from './components/ClosingSection.vue'
</script>

<template>
  <a
    href="#main"
    class="sr-only z-[60] rounded-full bg-white px-5 py-3 font-semibold text-ink-950 focus:not-sr-only focus:fixed focus:top-3 focus:left-3"
  >
    Skip to content
  </a>
  <SiteHeader />
  <main id="main" tabindex="-1" class="outline-none">
    <HeroSection />
    <FeatureSection />
    <WorkflowSection />
    <TrustSection />
    <FaqSection />
    <ClosingSection />
  </main>
</template>
