/*
 * Frozen-fixture oracle for the safe Rust ELDC port.
 *
 * This tool is outside the Rust build. build-c-oracle.sh compiles it against
 * the pinned upstream source. It reads one UTF-8 sample per line and emits a
 * tab-separated result for generate-parity-fixture.py.
 */

#ifndef ELD_CORE_PATH
#error "ELD_CORE_PATH must name the pinned upstream eld_core.c"
#endif

#include ELD_CORE_PATH

#define INPUT_BUFFER_SIZE 8192

static const int selected_indexes[] = {
    1, 9, 11, 12, 17, 20, 25, 26, 29, 36, 42, 44, 54, 57, 59,
};

static uint64_t selected_mask(void)
{
    uint64_t mask = 0;
    size_t count = sizeof selected_indexes / sizeof selected_indexes[0];
    for (size_t index = 0; index < count; index++) {
        mask |= UINT64_C(1) << selected_indexes[index];
    }
    return mask;
}

static void emit_result(const char *text, uint64_t mask)
{
    float raw[MAX_LANGUAGES];
    int feature_count = 0;
    detect_ex(text, raw, &feature_count, NULL);
    (void)raw;

    EldConfig config = {
        ELD_MAX_SCORES,
        1,
        SCHEME_ISO639_1,
        mask,
        1,
    };
    EldResult result;
    eld_process_line(text, &config, &result);

    float top_score = result.n_entries > 0 ? result.entries[0].ns : 0.0f;
    float second_score = result.n_entries > 1 ? result.entries[1].ns : 0.0f;
    const char *language = result.language != NULL ? result.language : "und";

    printf(
        "%s\t%d\t%d\t%.9g\t%.9g\t%d",
        language,
        result.reliable,
        feature_count,
        top_score,
        second_score,
        result.n_entries
    );
    for (int index = 0; index < result.n_entries; index++) {
        printf(
            "\t%s\t%.9g",
            ELD_langCodes[result.entries[index].idx],
            result.entries[index].ns
        );
    }
    putchar('\n');
}

int main(void)
{
    char input[INPUT_BUFFER_SIZE];
    uint64_t mask = selected_mask();
    init_detector();

    while (fgets(input, sizeof input, stdin) != NULL) {
        size_t length = strlen(input);
        while (length > 0 && (input[length - 1] == '\n' || input[length - 1] == '\r')) {
            input[--length] = '\0';
        }
        emit_result(input, mask);
    }
    return ferror(stdin) ? 1 : 0;
}
