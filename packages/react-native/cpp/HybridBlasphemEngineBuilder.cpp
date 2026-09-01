#include "HybridBlasphemEngineBuilder.hpp"

#include <stdexcept>

#include "HybridBlasphemEngine.hpp"

namespace margelo::nitro::blasphem {

HybridBlasphemEngineBuilder::~HybridBlasphemEngineBuilder() {
  if (builder_ != nullptr) {
    blasphem_builder_free(builder_);
    builder_ = nullptr;
  }
}

void HybridBlasphemEngineBuilder::configure(bool detectLanguage, bool grawlix) {
  if (builder_ != nullptr) {
    blasphem_builder_free(builder_);
  }
  builder_ = blasphem_builder_new(detectLanguage, grawlix);
}

void HybridBlasphemEngineBuilder::add(const std::string& locale, const std::shared_ptr<ArrayBuffer>& pack,
                                      const std::optional<std::string>& packSha256,
                                      const std::optional<std::shared_ptr<ArrayBuffer>>& detect,
                                      const std::optional<std::string>& detectSha256) {
  ensureBuilder();
  if (pack == nullptr) {
    throw std::runtime_error("BLASPHEM_PACK_INVALID: " + locale + ".pack bytes are required");
  }
  const uint8_t* detectData = nullptr;
  size_t detectLength = 0;
  if (detect.has_value() && *detect != nullptr) {
    detectData = (*detect)->data();
    detectLength = (*detect)->size();
  }
  int32_t status = blasphem_builder_add(builder_, locale.c_str(), pack->data(), pack->size(),
                                        packSha256.has_value() ? packSha256->c_str() : nullptr, detectData, detectLength,
                                        detectSha256.has_value() ? detectSha256->c_str() : nullptr);
  if (status != 0) {
    throw std::runtime_error(builderError());
  }
}

std::shared_ptr<HybridBlasphemEngineSpec> HybridBlasphemEngineBuilder::build() {
  ensureBuilder();
  blasphem_engine* engine = blasphem_builder_build(builder_);
  if (engine == nullptr) {
    // The builder survives a failed build so its error can be read; drop it afterwards.
    std::string message = builderError();
    blasphem_builder_free(builder_);
    builder_ = nullptr;
    throw std::runtime_error(message);
  }
  builder_ = nullptr; // consumed by the successful build
  return std::make_shared<HybridBlasphemEngine>(engine);
}

void HybridBlasphemEngineBuilder::ensureBuilder() {
  if (builder_ == nullptr) {
    builder_ = blasphem_builder_new(true, false);
  }
}

std::string HybridBlasphemEngineBuilder::builderError() const {
  const char* message = builder_ != nullptr ? blasphem_builder_error(builder_) : nullptr;
  if (message == nullptr) {
    message = blasphem_last_error();
  }
  return message != nullptr ? std::string(message) : std::string("BLASPHEM_PACK_INVALID: unknown native error");
}

} // namespace margelo::nitro::blasphem
