#import <UIKit/UIKit.h>
#include <stdbool.h>

bool scoop_os_ios_protected_data_available(void) {
    return UIApplication.sharedApplication.protectedDataAvailable;
}
