#pragma once

#include "matches.h"
#include "sidebar.h"
#include "sidebarwidget.h"

enum WARPWidgetType
{
    // TODO: IDK even know what to call this...
    PossibleFunctions = 0,
};

class WARPSidebarWidget : public SidebarWidget
{
    Q_OBJECT
    BinaryNinja::Ref<BinaryNinja::Logger> m_logger;
    BinaryViewRef m_data;
    ViewFrame *m_currentFrame;
    WARPMatchesWidget* m_matchesWidget;

public:
    explicit WARPSidebarWidget(BinaryViewRef data);

    ~WARPSidebarWidget() override;

    void notifyViewChanged(ViewFrame *) override;

    void notifyViewLocationChanged(View *, const ViewLocation &) override;
};

class WARPSidebarWidgetType : public SidebarWidgetType
{
public:
    WARPSidebarWidgetType();

    SidebarWidgetLocation defaultLocation() const override { return SidebarWidgetLocation::RightContent; }
    SidebarContextSensitivity contextSensitivity() const override { return PerViewTypeSidebarContext; }

    WARPSidebarWidget *createWidget(ViewFrame *viewFrame, BinaryViewRef data) override
    {
        return new WARPSidebarWidget(data);
    }
};
