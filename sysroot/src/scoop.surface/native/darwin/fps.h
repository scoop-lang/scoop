#pragma once
#import <Foundation/Foundation.h>
#import <Metal/Metal.h>

@interface SSUMetalFpsProbe : NSObject
+ (instancetype)probeForWindowNumber:(NSInteger)windowNumber;
- (instancetype)initWithWindowNumber:(NSInteger)windowNumber;
- (void)watchDrawable:(id<MTLDrawable>)drawable;
- (void)finish;
@end
