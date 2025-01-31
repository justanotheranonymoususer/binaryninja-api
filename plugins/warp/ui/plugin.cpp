#include "plugin.h"

#include "matches.h"
#include "warp.h"
#include "viewframe.h"

using namespace BinaryNinja;


WARPSidebarWidget::WARPSidebarWidget(BinaryViewRef data) : SidebarWidget("WARP"), m_data(data)
{
	m_logger = LogRegistry::CreateLogger("WARPUI");
	auto tabWidget = new QTabWidget(this);
	QVBoxLayout *layout = new QVBoxLayout(this);
	layout->setContentsMargins(0, 0, 0, 0);
	layout->setSpacing(0);

	// TODO: This probably should be initialized to something lmao.
	m_currentFrame = nullptr;

	QFrame* matchesFrame = new QFrame(this);
	m_matchesWidget = new WARPMatchesWidget(nullptr);
	QVBoxLayout* matchesLayout = new QVBoxLayout();
	matchesLayout->setContentsMargins(0, 0, 0, 0);
	matchesLayout->setSpacing(0);
	matchesLayout->addWidget(m_matchesWidget);
	matchesFrame->setLayout(matchesLayout);

	tabWidget->addTab(matchesFrame, "Possible Functions");
	layout->addWidget(tabWidget);
	this->setLayout(layout);
}


WARPSidebarWidget::~WARPSidebarWidget()
{
	// TODO: I am sure we will need to do something here.
}


void WARPSidebarWidget::notifyViewChanged(ViewFrame *view)
{
	if (!view)
		return;

	if (view == m_currentFrame)
		return;
	m_currentFrame = view;

	// TODO: We need to set some stuff here prolly.
}


void WARPSidebarWidget::notifyViewLocationChanged(View *view, const ViewLocation &location)
{
	auto function = location.getFunction();
	if (function != nullptr)
	{
		// TODO: The sidebar widget will have a function ref prolly, just update it.
		auto guid = BNWARPGetFunctionGUID(function->m_object);
		if (!guid)
		{
			LogInfo("No GUID for current function");
			return;
		}
		LogInfo("Function GUID: %s", guid);

		LogInfo("Function changed");
		// We have navigated to a new function, we should set the current function.
		m_matchesWidget->SetCurrentFunction(function);
		// We should also update the matches duh.
		m_matchesWidget->UpdateMatches();
	}

	// // Update matches widget
	// if (function != m_matchesWidget->GetCurrentFunction())
	// {
	// 	LogInfo("Function changed");
	// 	// We have navigated to a new function, we should set the current function.
	// 	m_matchesWidget->SetCurrentFunction(function);
	// 	// We should also update the matches duh.
	// 	m_matchesWidget->UpdateMatches();
	// }
}


WARPSidebarWidgetType::WARPSidebarWidgetType() :
	SidebarWidgetType(QImage(":/icons/images/letters/letter-W.png"), "WARP")
{}


extern "C" {
	BN_DECLARE_UI_ABI_VERSION

	BINARYNINJAPLUGIN void CorePluginDependencies()
	{
		// We must have WARP to enable this plugin of course!
		AddRequiredPluginDependency("warp_ninja");
	}

	BINARYNINJAPLUGIN bool UIPluginInit()
	{
		LogInfo("Initializing WARP UI plugin");
		Sidebar::addSidebarWidgetType(new WARPSidebarWidgetType());
		return true;
	}
}
