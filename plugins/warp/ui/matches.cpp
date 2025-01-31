#include <QGridLayout>

#include "matches.h"

#include <QHeaderView>
#include <QStyledItemDelegate>

#include "symbollist.h"
#include "theme.h"
#include "warp.h"

WARPMatchesWidget::WARPMatchesWidget(FunctionRef current)
{
    // NOTE: Might be nullptr if the no selected function.
    m_current = current;

    // Create the QT stuff
    QGridLayout* layout = new QGridLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(2);
    auto newPalette = palette();
    newPalette.setColor(QPalette::Window, getThemeColor(SidebarWidgetBackgroundColor));
    setAutoFillBackground(true);
    setPalette(newPalette);

    // TODO: Symbol, and some metrics for like, how many matching constraints etc...
    m_matchesTable = new WARPMatchesTableView({"Symbol"}, this);
    m_matchesModel = m_matchesTable->GetModel();
    m_proxyModel = new WARPMatchesFilterModel(this);
    m_proxyModel->setSourceModel(m_matchesModel);
    // TODO: For symbol do we have a symbol item delegate? That way it shows up better.
    m_matchesTable->setItemDelegateForColumn(0, new SymbolListDelegate(this));
    m_matchesTable->setSelectionMode(QTableView::SingleSelection);

    // Create filter stuff
    m_filterEdit = new FilterEdit(this);
    m_filterView = new FilteredView(this, m_matchesTable, this, m_filterEdit);
    m_filterView->setFilterPlaceholderText("Search possible matches");

    // Set the layout stuff
    layout->addWidget(m_filterEdit, 0, 0, 1, 5);

    m_splitter = new QSplitter(Qt::Horizontal);
    m_splitter->addWidget(m_matchesTable);
    // TODO: Add a Widget that shows all the information of a single matched function.
    // m_splitter->addWidget(m_typeEditor);
    
    layout->addWidget(m_splitter, 1, 0, 1, 5);
    setLayout(layout);

    // Setup some interaction callbacks
    connect(m_matchesTable, &WARPMatchesTableView::clicked, this, [=](const QModelIndex& index) {
        return; // TODO: Clicked item should give us some extra data in another widget below the table.
        // TODO: The widget below the table should show the type?
        // TODO: Set selectedMatch
        // TODO: If we want to use the type editor, we would need to have a type container.
        
    });
    
    connect(m_matchesTable, &WARPMatchesTableView::doubleClicked, this, [=](const QModelIndex& index) {
        if (m_current == nullptr)
            return;


        // Get the selected row for the given index
        if (!index.isValid())
            return;

        int selectedRow = index.row();
        BNWARPFunction* previous = BNWARPSetMatchedFunction(m_current->m_object, m_matchesModel->m_functions[selectedRow]);
        if (previous)
        {
            BNWARPFreeFunction(previous);
        }
        // So it shows visually as selected.
        UpdateMatches();
    });
}

void WARPMatchesWidget::setFilter(const std::string& filter)
{
    m_proxyModel->setFilterFixedString(QString::fromStdString(filter));
    m_filterView->showFilter(QString::fromStdString(filter));
}

bool WARPMatchesFilterModel::filterAcceptsRow(int sourceRow, const QModelIndex &sourceParent) const
{
    auto filterString = filterRegularExpression().pattern();
    if (filterString.isEmpty())
        return true;

    for (int i = 0; i < sourceModel()->columnCount(); i++)
    {
        auto index = sourceModel()->index(sourceRow, i, sourceParent);
        auto data = QRegularExpression::escape(index.data().toString());
        if (data.contains(filterString, Qt::CaseInsensitive))
            return true;
    }

    return false;
}

bool WARPMatchesFilterModel::lessThan(const QModelIndex &sourceLeft, const QModelIndex &sourceRight) const
{
    auto leftData = sourceLeft.data().toString();
    auto rightData = sourceRight.data().toString();
    return QString::localeAwareCompare(leftData, rightData) < 0;
}

WARPMatchedFunctionItemModel::WARPMatchedFunctionItemModel(const QStringList &labels, QObject *parent)
{
    this->setHorizontalHeaderLabels(labels);
}

void WARPMatchedFunctionItemModel::Refresh()
{
    beginResetModel();
    setRowCount(0);
    for (auto& row : m_rows)
        appendRow(row);
    endResetModel();
}

WARPMatchesTableView::WARPMatchesTableView(const QStringList &labels, QWidget *parent)
{
    m_model = new WARPMatchedFunctionItemModel(labels, parent);
    this->setModel(m_model);
    this->horizontalHeader()->setStretchLastSection(true);
    this->verticalHeader()->hide();
    this->setSelectionBehavior(QAbstractItemView::SelectRows);
    this->setSelectionMode(QAbstractItemView::SingleSelection);
    this->setEditTriggers(QAbstractItemView::NoEditTriggers);
    this->setFocusPolicy(Qt::NoFocus);
    this->setShowGrid(false);
    this->setAlternatingRowColors(true);
    this->setSortingEnabled(true);
}

void WARPMatchesWidget::UpdateMatches()
{
    // The caller wants us to update the matches table, so do it!

    // Clear matches as they are no longer valid.
    m_matchesModel->Clear();
    m_matchesModel->setRowCount(0);

    // Temporarily disable sorting so we can add rows faster
    m_matchesTable->setModel(m_matchesModel);
    m_matchesTable->setSortingEnabled(false);

    // Disable interactions while we are updating matches...
    // TODO: What about when we are lazy loading? I guess then we have a bunch of callbacks so...
    m_matchesTable->setEnabled(false);

    if (!m_current)
    {
        // TODO: We need to have a display that says no selected function or something.
        BinaryNinja::LogInfo("No current function");
        return;
    }

    auto guid = BNWARPGetFunctionGUID(m_current->m_object);
    if (!guid)
    {
        BinaryNinja::LogInfo("No GUID for current function");
        return;
    }
    m_matchesModel->m_functions = BNWARPGetPossibleFunctions(m_current->GetPlatform()->m_object, guid,  &m_matchesModel->m_functionCount);
    BNFreeString(guid);
    BNWARPFunction* matchedFunction = BNWARPGetMatchedFunction(m_current->m_object);

    // TODO: This is the best we can do _until_ functions can have an associated UUID.
    std::optional<std::string> matchedFunctionName = {};
    if (matchedFunction)
    {
        BNSymbol* symCore = BNWARPGetFunctionSymbol(m_current->m_object, matchedFunction);
        auto sym = BinaryNinja::Symbol(symCore);
        matchedFunctionName = sym.GetRawName();
        BNFreeSymbol(symCore);
        BNWARPFreeFunction(matchedFunction);
    }

    for (int i = 0; i < m_matchesModel->m_functionCount; i++)
    {
        // TODO: Sym is leaked lmao
        BNSymbol* symCore = BNWARPGetFunctionSymbol(m_current->m_object, m_matchesModel->m_functions[i]);
        auto sym = BinaryNinja::Symbol(symCore);
        auto symName = sym.GetShortName();
        QString symNameStr = QString::fromStdString(symName);
        auto name = new QStandardItem(symNameStr);
        QList<QStandardItem*> row = {name};

        // Highlight the matched function (if any)
        // TODO: The way we identify this is awful.
        if (matchedFunctionName.has_value() &&
            matchedFunctionName.value() == sym.GetRawName())
        {
            // Bright yellow really ties this UI together.
            name->setBackground(QColor(255, 255, 0));
        }

        BNFreeSymbol(symCore);
        m_matchesModel->AddRow(row);
    }

    // We are done, re-enable table.
    m_matchesTable->setEnabled(true);

    // Do some other stuff i guess.
    m_matchesModel->Refresh();
    m_matchesTable->setModel(m_proxyModel);
    m_matchesTable->setSortingEnabled(true);
}


