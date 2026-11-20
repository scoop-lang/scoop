volatile char live_data[] = "scoop-live-native-section";
volatile char unused_table[65536] = "scoop-unused-native-table";
static volatile char unused_message[] = "scoop-data-for-unused-function";

int unused_function(unsigned int index) {
    return unused_message[index % sizeof(unused_message)];
}

static int increment(int value) {
    return value + 1;
}

static int (*volatile callback)(int) = increment;

int read_live_data(void) {
    return callback(live_data[0] == 's' ? 41 : 0);
}
