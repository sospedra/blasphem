#pragma once

#include "HybridBlasphemEngineBuilderSpec.hpp"
#include "blasphem.h"

namespace margelo::nitro::blasphem {

/**
 * Collects one locale at a time, then builds the engine. Nitro creates this
 * object without constructor arguments, so `configure` sets the options; a
 * builder that was never configured detects language and skips grawlix.
 */
class HybridBlasphemEngineBuilder : public HybridBlasphemEngineBuilderSpec {
public:
  HybridBlasphemEngineBuilder() : HybridObject(TAG) {}
  ~HybridBlasphemEngineBuilder() override;

  void configure(bool detectLanguage, bool grawlix) override;
  void add(const std::string& locale, const std::shared_ptr<ArrayBuffer>& pack, const std::optional<std::string>& packSha256,
           const std::optional<std::shared_ptr<ArrayBuffer>>& detect, const std::optional<std::string>& detectSha256) override;
  std::shared_ptr<HybridBlasphemEngineSpec> build() override;

private:
  void ensureBuilder();
  std::string builderError() const;

  blasphem_builder* builder_ = nullptr;
};

} // namespace margelo::nitro::blasphem
