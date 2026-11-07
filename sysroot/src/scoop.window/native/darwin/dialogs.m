#include "../dialogs.h"
#import "window.h"
#import <UniformTypeIdentifiers/UniformTypeIdentifiers.h>
#include <errno.h>
#include <string.h>

@interface SWFileDialog : NSObject
@property(nonatomic, strong) SWApplication *application;
@property(nonatomic, strong) SWWindow *owner;
@property(nonatomic, strong) NSSavePanel *panel;
@property(nonatomic, strong) NSMutableArray<UTType *> *types;
@property(nonatomic, strong) NSMutableArray<NSData *> *paths;
@property(nonatomic, strong) NSData *domain;
@property(nonatomic) int kind, outcome;
@property(nonatomic) int64_t error;
@property(nonatomic) BOOL started, cancelled, running, failed;
@end
@implementation SWFileDialog
@end

static void fail(SWFileDialog *session, int64_t code, NSString *domain) {
  session.error = code;
  session.domain = [domain dataUsingEncoding:NSUTF8StringEncoding];
  session.outcome = 3;
}

static void finish(SWFileDialog *session, NSModalResponse response) {
  [session.panel orderOut:nil];
  if (session.failed) {
    session.outcome = 3;
  } else if (session.cancelled || response == NSModalResponseCancel) {
    session.outcome = 2;
  } else if (response != NSModalResponseOK) {
    fail(session, response, @"AppKit");
  } else {
    NSArray<NSURL *> *urls =
        session.kind == 2 ? (session.panel.URL ? @[ session.panel.URL ] : @[])
                          : ((NSOpenPanel *)session.panel).URLs;
    session.paths = [NSMutableArray new];
    for (NSURL *url in urls) {
      const char *path = url.isFileURL ? url.fileSystemRepresentation : NULL;
      if (!path) {
        fail(session, -1, nil);
        break;
      }
      [session.paths addObject:[NSData dataWithBytes:path length:strlen(path)]];
    }
    if (!session.outcome) {
      if (!session.paths.count)
        fail(session, 1, @"AppKit");
      else
        session.outcome = 1;
    }
  }
  sw_modal_resume_ending(session.application);
  sw_wake(session.application);
}

SWResult scoop_window_file_create(void *application, void *owner,
                                  int32_t kind) {
  @try {
    SWFileDialog *session = [SWFileDialog new];
    session.application = (__bridge SWApplication *)application;
    session.owner = (__bridge SWWindow *)owner;
    session.kind = kind;
    session.types = [NSMutableArray new];
    session.panel =
        kind == 2 ? [NSSavePanel savePanel] : [NSOpenPanel openPanel];
    if (!session.panel)
      return (SWResult){NULL, -2};
    if (kind != 2) {
      NSOpenPanel *panel = (NSOpenPanel *)session.panel;
      panel.canChooseFiles = kind != 3;
      panel.canChooseDirectories = kind == 3;
      panel.allowsMultipleSelection = kind == 1;
    }
    return (SWResult){(__bridge_retained void *)session, 0};
  } @catch (NSException *exception) {
    return (SWResult){NULL, 1};
  }
}

int64_t scoop_window_file_option(void *pointer, int32_t field,
                                 const uint8_t *data, int64_t length) {
  SWFileDialog *session = (__bridge SWFileDialog *)pointer;
  @try {
    if (field == 0 || field == 3) {
      NSString *text = [[NSString alloc] initWithBytes:data
                                                length:(NSUInteger)length
                                              encoding:NSUTF8StringEncoding];
      if (!text)
        return -1;
      if (!field)
        session.panel.title = text;
      else {
        UTType *type = [UTType typeWithFilenameExtension:text
                                        conformingToType:UTTypeData];
        if (!type)
          return -1;
        [session.types addObject:type];
        session.panel.allowedContentTypes = session.types;
        session.panel.allowsOtherFileTypes = NO;
      }
    } else if (field == 1) {
      NSMutableData *terminated =
          [NSMutableData dataWithBytes:data length:(NSUInteger)length];
      [terminated increaseLengthBy:1];
      NSURL *url = [NSURL fileURLWithFileSystemRepresentation:terminated.bytes
                                                  isDirectory:YES
                                                relativeToURL:nil];
      if (!url)
        return -1;
      session.panel.directoryURL = url;
    } else if (field == 2) {
      NSString *name = [[NSFileManager defaultManager]
          stringWithFileSystemRepresentation:(const char *)data
                                      length:(NSUInteger)length];
      if (!name)
        return -1;
      session.panel.nameFieldStringValue = name;
    } else
      return -4;
    return 0;
  } @catch (NSException *exception) {
    return 1;
  }
}

void scoop_window_file_start(void *pointer) {
  SWFileDialog *session = (__bridge SWFileDialog *)pointer;
  if (session.outcome)
    return;
  session.started = YES;
  @try {
    if (session.owner) {
      if (session.owner.window.attachedSheet) {
        fail(session, -5, nil);
        sw_wake(session.application);
        return;
      }
      [session.panel beginSheetModalForWindow:session.owner.window
                            completionHandler:^(NSModalResponse response) {
                              finish(session, response);
                            }];
    } else {
      session.running = YES;
      NSModalResponse response;
      @try {
        response = [session.panel runModal];
      } @finally {
        session.running = NO;
      }
      finish(session, response);
    }
  } @catch (NSException *exception) {
    if (session.panel.sheetParent) {
      session.failed = YES;
      session.error = 1;
      session.domain = [exception.name dataUsingEncoding:NSUTF8StringEncoding];
      [session.panel.sheetParent endSheet:session.panel
                               returnCode:NSModalResponseAbort];
    } else {
      [session.panel orderOut:nil];
      fail(session, 1, exception.name);
      sw_modal_resume_ending(session.application);
    }
    sw_wake(session.application);
  }
}

int32_t scoop_window_file_cancel(void *pointer) {
  SWFileDialog *session = (__bridge SWFileDialog *)pointer;
  if (session.outcome || session.cancelled || session.failed)
    return 0;
  session.cancelled = YES;
  if (!session.started)
    session.outcome = 2;
  else
    [session.panel cancel:nil];
  sw_wake(session.application);
  return 1;
}

SWFileResult scoop_window_file_result(void *pointer) {
  SWFileDialog *session = (__bridge SWFileDialog *)pointer;
  return (SWFileResult){session.outcome, session.error,
                        (int64_t)session.paths.count, session.domain.bytes,
                        (int64_t)session.domain.length};
}
SWFilePath scoop_window_file_path(void *pointer, int64_t index) {
  NSData *path = ((__bridge SWFileDialog *)pointer).paths[(NSUInteger)index];
  return (SWFilePath){path.bytes, (int64_t)path.length};
}
void scoop_window_file_release(void *pointer) {
  (void)CFBridgingRelease(pointer);
}
