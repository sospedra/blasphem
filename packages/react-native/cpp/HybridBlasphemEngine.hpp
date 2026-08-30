#pragma once

#include "HybridBlasphemEngineSpec.hpp"
#include "blasphem.h"

namespace margelo::nitro::blasphem {

/** Owns one native engine. `judge` is synchronous over JSI. */
class HybridBlasphemEngine : public HybridBlasphemEngineSpec {
public:
  explicit HybridBlasphemEngine(blasphem_engine* engine) : HybridObject(TAG), engine_(engine) {}
  ~HybridBlasphemEngine() override { close(); }

  std::vector<std::string> getLocales() override;
  NativeJudgement judge(const std::string& text) override;
  void close() override;

private:
  blasphem_engine* engine_;
};

} // namespace margelo::nitro::blasphem
