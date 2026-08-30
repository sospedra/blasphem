#include "HybridBlasphemEngine.hpp"

#include <stdexcept>

namespace margelo::nitro::blasphem {

namespace {

std::optional<std::string> takeText(char* text) {
  if (text == nullptr) {
    return std::nullopt;
  }
  return std::string(text);
}

} // namespace

std::vector<std::string> HybridBlasphemEngine::getLocales() {
  std::vector<std::string> locales;
  if (engine_ == nullptr) {
    return locales;
  }
  size_t count = blasphem_engine_locale_count(engine_);
  locales.reserve(count);
  for (size_t index = 0; index < count; index += 1) {
    char* code = blasphem_engine_locale(engine_, index);
    if (code != nullptr) {
      locales.emplace_back(code);
      blasphem_text_free(code);
    }
  }
  return locales;
}

NativeJudgement HybridBlasphemEngine::judge(const std::string& text) {
  if (engine_ == nullptr) {
    throw std::runtime_error("BLASPHEM_CLOSED: the judge was closed");
  }
  blasphem_judgement verdict = blasphem_engine_judge(engine_, text.c_str());
  NativeJudgement result(verdict.safe, verdict.score, takeText(verdict.locale), takeText(verdict.grawlix));
  blasphem_judgement_free(verdict);
  return result;
}

void HybridBlasphemEngine::close() {
  if (engine_ != nullptr) {
    blasphem_engine_free(engine_);
    engine_ = nullptr;
  }
}

} // namespace margelo::nitro::blasphem
