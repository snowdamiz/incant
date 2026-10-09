<script setup lang="ts">
import SiteHeader from './components/SiteHeader.vue'
import HeroSection from './components/HeroSection.vue'
import PrinciplesSection from './components/PrinciplesSection.vue'
import StatementBand from './components/StatementBand.vue'
import FaqSection from './components/FaqSection.vue'
import SiteFooter from './components/SiteFooter.vue'
</script>

<template>
  <a
    href="#main"
    class="sr-only z-[60] rounded-md bg-ink px-5 py-3 font-medium text-paper focus:not-sr-only focus:fixed focus:top-3 focus:left-3"
  >
    Skip to content
  </a>
  <SiteHeader />
  <main id="main" tabindex="-1" class="outline-none">
    <HeroSection />
    <PrinciplesSection />
    <StatementBand />
    <FaqSection />
  </main>
  <SiteFooter />
</template>
