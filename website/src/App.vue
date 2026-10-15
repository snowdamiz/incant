<script setup lang="ts">
import SiteHeader from './components/SiteHeader.vue'
import HeroSection from './components/HeroSection.vue'
import EditorSection from './components/EditorSection.vue'
import PrinciplesSection from './components/PrinciplesSection.vue'
import ModelingSection from './components/ModelingSection.vue'
import ToolsetSection from './components/ToolsetSection.vue'
import ScriptSection from './components/ScriptSection.vue'
import ShipSection from './components/ShipSection.vue'
import FaqSection from './components/FaqSection.vue'
import SiteFooter from './components/SiteFooter.vue'
</script>

<template>
  <a
    href="#main"
    class="sr-only z-[60] rounded-md bg-fg px-5 py-3 font-medium text-night focus:not-sr-only focus:fixed focus:top-3 focus:left-3"
  >
    Skip to content
  </a>
  <SiteHeader />
  <main id="main" tabindex="-1" class="outline-none">
    <HeroSection />
    <EditorSection />
    <PrinciplesSection />
    <ModelingSection />
    <ToolsetSection />
    <ScriptSection />
    <ShipSection />
    <FaqSection />
  </main>
  <SiteFooter />
</template>
